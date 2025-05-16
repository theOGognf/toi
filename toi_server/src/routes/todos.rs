use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        state::ToiState,
        todos::{CompleteTodoRequest, NewTodo, NewTodoRequest, Todo, TodoQueryParams},
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user's query, find todo items similar to the one that the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(
            add_todo,
            complete_matching_todos,
            delete_matching_todos,
            get_matching_todos
        ))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /todos extensions
    let add_todo_json_schema = schema_for!(NewTodoRequest);
    let add_todo_json_schema =
        serde_json::to_value(add_todo_json_schema).expect("schema unserializable");
    let add_todo_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", add_todo_json_schema)
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(add_todo_extensions);

    // Update PUT /todos extensions
    let complete_todo_json_schema = schema_for!(CompleteTodoRequest);
    let complete_todo_json_schema =
        serde_json::to_value(complete_todo_json_schema).expect("schema unserializable");
    let complete_todo_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", complete_todo_json_schema)
        .build();
    paths
        .put
        .as_mut()
        .expect("PUT doesn't exist")
        .extensions
        .get_or_insert(complete_todo_extensions);

    // Update DELETE and GET /todos extensions
    let todos_json_schema = schema_for!(TodoQueryParams);
    let todos_json_schema = serde_json::to_value(todos_json_schema).expect("schema unserializable");
    let todos_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", todos_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(todos_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(todos_extensions);

    router
}

async fn search_todos(
    state: &ToiState,
    params: &TodoQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let mut query = schema::todos::table.select(Todo::as_select()).into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = params.created_from {
        query = query.filter(schema::todos::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = params.created_to {
        query = query.filter(schema::todos::created_at.le(created_to));
    }

    // Filter todos completed on or after date.
    if let Some(completed_from) = params.completed_from {
        query = query.filter(schema::todos::completed_at.ge(completed_from));
    }

    // Filter todos completed on or before date.
    if let Some(completed_to) = params.completed_to {
        query = query.filter(schema::todos::completed_at.le(completed_to));
    }

    // Filter incomplete todos.
    if let Some(scope) = &params.incomplete {
        match scope {
            utils::Scope::In => query = query.or_filter(schema::todos::completed_at.is_null()),
            utils::Scope::Out => query = query.filter(schema::todos::completed_at.is_not_null()),
        }
    }

    // Filter never due todos.
    if let Some(scope) = &params.never_due {
        match scope {
            utils::Scope::In => query = query.or_filter(schema::todos::due_at.is_null()),
            utils::Scope::Out => query = query.filter(schema::todos::due_at.is_not_null()),
        }
    }

    // Order items.
    match params.order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::todos::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::todos::created_at.desc()),
        None => {
            // By default, filter items similar to a given query.
            if let Some(similarity_search_params) = &params.similarity_search_params {
                let input = EmbeddingPromptTemplate::builder()
                    .instruction_prefix(INSTRUCTION_PREFIX.to_string())
                    .query_prefix(QUERY_PREFIX.to_string())
                    .build()
                    .apply(&similarity_search_params.query);
                let embedding_request = EmbeddingRequest { input };
                let embedding = state.model_client.embed(embedding_request).await?;
                query = query
                    .filter(
                        schema::todos::embedding
                            .l2_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::todos::embedding.l2_distance(embedding));
            }
        }
    }

    // Limit number of items.
    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    // Get all the items that match the query.
    let todos: Vec<Todo> = query.load(conn).await.map_err(utils::diesel_error)?;
    if todos.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no todos found".to_string()));
    }

    let (ids, documents): (Vec<i32>, Vec<String>) =
        todos.into_iter().map(|todo| (todo.id, todo.item)).unzip();

    // Rerank and filter items once more.
    let ids = if let Some(similarity_search_params) = &params.similarity_search_params {
        if similarity_search_params.use_reranking_filter {
            let rerank_request = RerankRequest {
                query: similarity_search_params.query.clone(),
                documents,
            };
            let rerank_response = state.model_client.rerank(rerank_request).await?;
            rerank_response
                .results
                .into_iter()
                .filter(|item| item.relevance_score >= state.server_config.similarity_threshold)
                .map(|item| ids[item.index])
                .collect()
        } else {
            ids
        }
    } else {
        ids
    };

    Ok(ids)
}

/// Add and return a todo.
///
/// Example queries for adding todos using this endpoint:
/// - Add a todo
/// - Make a task
/// - Add a task
#[utoipa::path(
    post,
    path = "",
    request_body = NewTodoRequest,
    responses(
        (status = 201, description = "Successfully added a todo", body = Todo),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_todo(
    State(state): State<ToiState>,
    Json(body): Json<NewTodoRequest>,
) -> Result<Json<Todo>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let NewTodoRequest {
        item,
        due_at,
        completed_at,
    } = body;
    let embedding_request = EmbeddingRequest {
        input: item.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let new_todo = NewTodo {
        item,
        embedding,
        due_at,
        completed_at,
    };
    let res = diesel::insert_into(schema::todos::table)
        .values(new_todo)
        .returning(Todo::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Complete and return todos.
///
/// Example queries for completing todos using this endpoint:
/// - Complete all todos
/// - Complete todos
/// - Update the todos
#[utoipa::path(
    put,
    path = "",
    request_body = CompleteTodoRequest,
    responses(
        (status = 200, description = "Successfully updated todos", body = Todo),
        (status = 404, description = "Todos not found")
    )
)]
#[axum::debug_handler]
pub async fn complete_matching_todos(
    State(state): State<ToiState>,
    Json(body): Json<CompleteTodoRequest>,
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let CompleteTodoRequest {
        completed_at,
        similarity_search_params,
        created_from,
        created_to,
        due_from,
        due_to,
        incomplete,
        never_due,
        order_by,
        limit,
    } = body;
    let params = TodoQueryParams {
        similarity_search_params,
        created_from,
        created_to,
        due_from,
        due_to,
        completed_from: None,
        completed_to: None,
        incomplete,
        never_due,
        order_by,
        limit,
    };
    let ids = search_todos(&state, &params, &mut conn).await?;
    let todos = diesel::update(schema::todos::table.filter(schema::todos::id.eq_any(ids)))
        .set(schema::todos::completed_at.eq(completed_at))
        .returning(Todo::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(todos))
}

/// Delete and return todos.
///
/// Example queries for deleting todos using this endpoint:
/// - Delete all todos
/// - Erase all todos
/// - Remove todos
/// - Delete as many todos
#[utoipa::path(
    delete,
    path = "",
    params(TodoQueryParams),
    responses(
        (status = 200, description = "Successfully deleted todos", body = [Todo]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_todos(
    State(state): State<ToiState>,
    Query(params): Query<TodoQueryParams>,
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_todos(&state, &params, &mut conn).await?;
    let todos = diesel::delete(schema::todos::table.filter(schema::todos::id.eq_any(ids)))
        .returning(Todo::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(todos))
}

/// Get todos.
///
/// Example queries for getting todos using this endpoint:
/// - Get all todos related to
/// - List all todos about
/// - What todos are there on
/// - How many todos are there about
#[utoipa::path(
    get,
    path = "",
    params(TodoQueryParams),
    responses(
        (status = 200, description = "Successfully got todos", body = [Todo]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_todos(
    State(state): State<ToiState>,
    Query(params): Query<TodoQueryParams>,
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_todos(&state, &params, &mut conn).await?;
    let todos = schema::todos::table
        .select(Todo::as_select())
        .filter(schema::todos::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(todos))
}
