use axum::{extract::State, http::StatusCode, response::Json};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        state::ToiState,
        todos::{CompleteTodoRequest, NewTodo, NewTodoRequest, Todo, TodoSearchParams},
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user's query, find todo items similar to the one that the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn todos_router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_todo, complete_matching_todos))
        .routes(routes!(delete_matching_todos))
        .routes(routes!(get_matching_todos))
        .with_state(state)
}

async fn search_todos(
    state: &ToiState,
    params: TodoSearchParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let TodoSearchParams {
        ids,
        query,
        use_reranking_filter,
        created_from,
        created_to,
        due_from,
        due_to,
        completed_from,
        completed_to,
        incomplete,
        never_due,
        order_by,
        limit,
    } = params;

    let mut sql_query = schema::todos::table.select(Todo::as_select()).into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = created_from {
        sql_query = sql_query.filter(schema::todos::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = created_to {
        sql_query = sql_query.filter(schema::todos::created_at.le(created_to));
    }

    // Filter items due on or after date.
    if let Some(due_from) = due_from {
        sql_query = sql_query.filter(schema::todos::due_at.ge(due_from));
    }

    // Filter items due on or before date.
    if let Some(due_to) = due_to {
        sql_query = sql_query.filter(schema::todos::due_at.le(due_to));
    }

    // Filter todos completed on or after date.
    if let Some(completed_from) = completed_from {
        sql_query = sql_query.filter(schema::todos::completed_at.ge(completed_from));
    }

    // Filter todos completed on or before date.
    if let Some(completed_to) = completed_to {
        sql_query = sql_query.filter(schema::todos::completed_at.le(completed_to));
    }

    // Filter incomplete todos.
    if let Some(scope) = incomplete {
        match scope {
            utils::Scope::In => sql_query = sql_query.filter(schema::todos::completed_at.is_null()),
            utils::Scope::Out => {
                sql_query = sql_query.filter(schema::todos::completed_at.is_not_null());
            }
        }
    }

    // Filter never due todos.
    if let Some(scope) = never_due {
        match scope {
            utils::Scope::In => sql_query = sql_query.filter(schema::todos::due_at.is_null()),
            utils::Scope::Out => sql_query = sql_query.filter(schema::todos::due_at.is_not_null()),
        }
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => sql_query = sql_query.order(schema::todos::created_at),
        Some(utils::OrderBy::Newest) => {
            sql_query = sql_query.order(schema::todos::created_at.desc());
        }
        None => {
            // By default, filter items similar to a given query.
            if let Some(ref query) = query {
                let input = EmbeddingPromptTemplate::builder()
                    .instruction_prefix(INSTRUCTION_PREFIX.to_string())
                    .query_prefix(QUERY_PREFIX.to_string())
                    .build()
                    .apply(query);
                let embedding_request = EmbeddingRequest { input };
                let embedding = state.model_client.embed(embedding_request).await?;
                sql_query = sql_query
                    .filter(
                        schema::todos::embedding
                            .cosine_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::todos::embedding.cosine_distance(embedding));
            }
        }
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        sql_query = sql_query.or_filter(schema::todos::id.eq_any(ids));
    }

    // Limit number of items.
    if let Some(limit) = limit {
        sql_query = sql_query.limit(limit);
    }

    // Get all the items that match the query.
    let todos: Vec<Todo> = sql_query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) =
        todos.into_iter().map(|todo| (todo.id, todo.item)).unzip();
    if ids.is_empty() {
        return Ok(ids);
    }

    // Rerank and filter items once more.
    let ids = match (query, use_reranking_filter) {
        (Some(query), Some(true)) => {
            let rerank_request = RerankRequest { query, documents };
            let rerank_response = state.model_client.rerank(rerank_request).await?;
            rerank_response
                .results
                .into_iter()
                .filter(|item| item.relevance_score >= state.server_config.similarity_threshold)
                .map(|item| ids[item.index])
                .collect()
        }
        _ => ids,
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
    extensions(
        ("x-json-schema-body" = json!(schema_for!(NewTodoRequest)))
    ),
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
    Json(params): Json<NewTodoRequest>,
) -> Result<Json<Todo>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let NewTodoRequest {
        item,
        due_at,
        completed_at,
    } = params;
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
    let result = diesel::insert_into(schema::todos::table)
        .values(new_todo)
        .returning(Todo::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(result))
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
    extensions(
        ("x-json-schema-body" = json!(schema_for!(CompleteTodoRequest)))
    ),
    request_body = CompleteTodoRequest,
    responses(
        (status = 200, description = "Successfully updated todos", body = Todo),
        (status = 404, description = "Todos not found")
    )
)]
#[axum::debug_handler]
pub async fn complete_matching_todos(
    State(state): State<ToiState>,
    Json(params): Json<CompleteTodoRequest>,
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let CompleteTodoRequest {
        ids,
        completed_at,
        query,
        use_reranking_filter,
        created_from,
        created_to,
        due_from,
        due_to,
        incomplete,
        never_due,
        order_by,
        limit,
    } = params;
    let params = TodoSearchParams {
        ids,
        query,
        use_reranking_filter,
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
    let ids = search_todos(&state, params, &mut conn).await?;
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
    post,
    path = "/delete",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(TodoSearchParams)))
    ),
    request_body = TodoSearchParams,
    responses(
        (status = 200, description = "Successfully deleted todos", body = [Todo]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No todos found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_todos(
    State(state): State<ToiState>,
    Json(params): Json<TodoSearchParams>,
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_todos(&state, params, &mut conn).await?;
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
    post,
    path = "/search",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(TodoSearchParams)))
    ),
    request_body = TodoSearchParams,
    responses(
        (status = 200, description = "Successfully got todos", body = [Todo]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No todos found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_todos(
    State(state): State<ToiState>,
    Json(params): Json<TodoSearchParams>,
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_todos(&state, params, &mut conn).await?;
    let todos = schema::todos::table
        .select(Todo::as_select())
        .filter(schema::todos::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(todos))
}
