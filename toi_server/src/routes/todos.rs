use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::EmbeddingRequest,
        state::ToiState,
        todos::{CompleteTodoRequest, NewTodo, NewTodoRequest, Todo, TodoQueryParams},
    },
    schema, utils,
};

pub fn router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_todo, delete_todo, get_todo))
        .routes(routes!(delete_matching_todos, get_matching_todos))
        .with_state(state)
}

/// Add a todo.
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
    Json(new_todo_request): Json<NewTodoRequest>,
) -> Result<Json<Todo>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let embedding_request = EmbeddingRequest {
        input: new_todo_request.item.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let new_todo = NewTodo {
        item: new_todo_request.item,
        embedding,
        due_at: new_todo_request.due_at,
        completed_at: new_todo_request.completed_at,
    };
    let res = diesel::insert_into(schema::todos::table)
        .values(new_todo)
        .returning(Todo::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Complete a todo by its database-generated ID.
#[utoipa::path(
    put,
    path = "/{id}",
    params(
        ("id" = i32, Path, description = "Database ID of todo to complete"),
        CompleteTodoRequest,
    ),
    responses(
        (status = 200, description = "Successfully updated todo", body = Todo),
        (status = 404, description = "Todo not found")
    )
)]
#[axum::debug_handler]
pub async fn complete_todo(
    State(pool): State<utils::Pool>,
    Path(id): Path<i32>,
    Query(params): Query<CompleteTodoRequest>,
) -> Result<Json<Todo>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let res = diesel::update(schema::todos::table)
        .set(schema::todos::completed_at.eq(params.completed_at))
        .filter(schema::todos::id.eq(id))
        .returning(Todo::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete a todo by its database-generated ID.
#[utoipa::path(
    delete,
    path = "/{id}",
    params(
        ("id" = i32, Path, description = "Database ID of todo to delete"),
    ),
    responses(
        (status = 200, description = "Successfully deleted todo"),
        (status = 404, description = "Todo not found")
    )
)]
#[axum::debug_handler]
pub async fn delete_todo(
    State(pool): State<utils::Pool>,
    Path(id): Path<i32>,
) -> Result<Json<Todo>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let res = diesel::delete(schema::todos::table)
        .filter(schema::todos::id.eq(id))
        .returning(Todo::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete todos that match search criteria.
///
/// Useful for deleting todos in bulk.
#[utoipa::path(
    delete,
    path = "/search",
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
    let mut query = schema::todos::table.select(schema::todos::id).into_boxed();

    // Filter todos similar to a query.
    if let Some(todo_similarity_search_params) = params.similarity_search_params {
        let embedding_request = EmbeddingRequest {
            input: todo_similarity_search_params.query,
        };
        let embedding = state.model_client.embed(embedding_request).await?;
        query = query.filter(
            schema::todos::embedding
                .cosine_distance(embedding.clone())
                .le(todo_similarity_search_params.distance_threshold),
        );

        // Sort by relevance.
        if params.order_by == Some(utils::OrderBy::Relevance) {
            query = query.order(schema::todos::embedding.l2_distance(embedding));
        }
    }

    // Filter todos created on or after date.
    if let Some(created_from) = params.created_from {
        query = query.filter(schema::todos::created_at.ge(created_from));
    }

    // Filter todos created on or before date.
    if let Some(created_to) = params.created_to {
        query = query.filter(schema::todos::created_at.le(created_to));
    }

    // Filter todos due on or after date.
    if let Some(due_from) = params.due_from {
        query = query.filter(schema::todos::due_at.ge(due_from));
    }

    // Filter todos due on or before date.
    if let Some(due_to) = params.due_to {
        query = query.filter(schema::todos::due_at.le(due_to));
    }

    // Filter todos completed on or after date.
    if let Some(completed_from) = params.completed_from {
        query = query.filter(schema::todos::completed_at.ge(completed_from));
    }

    // Filter todos completed on or before date.
    if let Some(completed_to) = params.completed_to {
        query = query.filter(schema::todos::completed_at.le(completed_to));
    }

    match params.order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::todos::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::todos::created_at.desc()),
        _ => {}
    }

    // Limit number of todos deleted.
    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    let res = diesel::delete(schema::todos::table.filter(schema::todos::id.eq_any(query)))
        .returning(Todo::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::internal_error)?;
    Ok(Json(res))
}

/// Get a todo by its database-generated ID.
#[utoipa::path(
    get,
    path = "/{id}",
    params(
        ("id" = i32, Path, description = "Database ID of todo to get"),
    ),
    responses(
        (status = 200, description = "Successfully got todo", body = Todo),
        (status = 404, description = "Todo not found")
    )
)]
#[axum::debug_handler]
pub async fn get_todo(
    State(pool): State<utils::Pool>,
    Path(id): Path<i32>,
) -> Result<Json<Todo>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let res = schema::todos::table
        .select(Todo::as_select())
        .filter(schema::todos::id.eq(id))
        .first(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Get todos that match search criteria.
///
/// Useful for getting todos in bulk.
#[utoipa::path(
    get,
    path = "/search",
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
    let mut query = schema::todos::table.select(Todo::as_select()).into_boxed();

    // Filter todos similar to a query.
    if let Some(todo_similarity_search_params) = params.similarity_search_params {
        let embedding_request = EmbeddingRequest {
            input: todo_similarity_search_params.query,
        };
        let embedding = state.model_client.embed(embedding_request).await?;
        query = query.filter(
            schema::todos::embedding
                .cosine_distance(embedding.clone())
                .le(todo_similarity_search_params.distance_threshold),
        );

        // Sort by relevance.
        if params.order_by == Some(utils::OrderBy::Relevance) {
            query = query.order(schema::todos::embedding.l2_distance(embedding));
        }
    }

    // Filter todos created on or after date.
    if let Some(created_from) = params.created_from {
        query = query.filter(schema::todos::created_at.ge(created_from));
    }

    // Filter todos created on or before date.
    if let Some(created_to) = params.created_to {
        query = query.filter(schema::todos::created_at.le(created_to));
    }

    // Filter todos due on or after date.
    if let Some(due_from) = params.due_from {
        query = query.filter(schema::todos::due_at.ge(due_from));
    }

    // Filter todos due on or before date.
    if let Some(due_to) = params.due_to {
        query = query.filter(schema::todos::due_at.le(due_to));
    }

    // Filter todos completed on or after date.
    if let Some(completed_from) = params.completed_from {
        query = query.filter(schema::todos::completed_at.ge(completed_from));
    }

    // Filter todos completed on or before date.
    if let Some(completed_to) = params.completed_to {
        query = query.filter(schema::todos::completed_at.le(completed_to));
    }

    match params.order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::todos::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::todos::created_at.desc()),
        _ => {}
    }

    // Limit number of todos returned.
    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    let res = query.load(&mut conn).await.map_err(utils::internal_error)?;
    Ok(Json(res))
}
