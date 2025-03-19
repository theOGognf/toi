use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{
    ExpressionMethods,
    prelude::{QueryDsl, SelectableHelper},
};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{models, schema, state, utils};

pub fn router(state: state::ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(
            add_note,
            delete_matching_notes,
            delete_note,
            get_matching_notes,
            get_note
        ))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "",
    responses(
        (status = 201, description = "Successfully added a note", body = models::notes::Note),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_note(
    State(state): State<state::ToiState>,
    Json(new_note_request): Json<models::notes::NewNoteRequest>,
) -> Result<Json<models::notes::Note>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let embedding_request = models::client::EmbeddingRequest {
        input: new_note_request.content.clone(),
    };
    let embedding = state.client.embed(embedding_request).await?;
    let new_note = models::notes::NewNote {
        content: new_note_request.content,
        embedding,
    };
    let res = diesel::insert_into(schema::notes::table)
        .values(new_note)
        .returning(models::notes::Note::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    params(
        ("id" = i32, Path, description = "Database ID of note to delete"),
    ),
    responses(
        (status = 200, description = "Successfully deleted note"),
        (status = 404, description = "Note not found")
    )
)]
#[axum::debug_handler]
pub async fn delete_note(
    State(pool): State<utils::Pool>,
    Path(id): Path<i32>,
) -> Result<Json<models::notes::Note>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let res = diesel::delete(schema::notes::table)
        .filter(schema::notes::id.eq(id))
        .returning(models::notes::Note::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

#[utoipa::path(
    delete,
    path = "",
    params(models::notes::NoteQueryParams),
    responses(
        (status = 200, description = "Successfully deleted notes", body = [models::notes::Note]),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_notes(
    State(state): State<state::ToiState>,
    Query(params): Query<models::notes::NoteQueryParams>,
) -> Result<Json<Vec<models::notes::Note>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let mut query = schema::notes::table.select(schema::notes::id).into_boxed();

    // Filter notes similar to a query.
    if let Some(note_similarity_search_params) = params.similarity_search_params {
        let embedding_request = models::client::EmbeddingRequest {
            input: note_similarity_search_params.query,
        };
        let embedding = state.client.embed(embedding_request).await?;
        query = query.filter(
            schema::notes::embedding
                .cosine_distance(embedding.clone())
                .le(note_similarity_search_params.distance_threshold),
        );

        // Sort by relevance.
        if params.order_by == Some(utils::OrderBy::Relevance) {
            query = query.order(schema::notes::embedding.l2_distance(embedding));
        }
    }

    // Filter notes created on or after date.
    if let Some(from) = params.from {
        query = query.filter(schema::notes::created_at.ge(from));
    }

    // Filter notes created on or before date.
    if let Some(to) = params.to {
        query = query.filter(schema::notes::created_at.le(to));
    }

    match params.order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::notes::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::notes::created_at.desc()),
        _ => {}
    }

    // Limit number of notes deleted.
    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    let res = diesel::delete(schema::notes::table.filter(schema::notes::id.eq_any(query)))
        .returning(models::notes::Note::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::internal_error)?;
    Ok(Json(res))
}

#[utoipa::path(
    get,
    path = "/{id}",
    params(
        ("id" = i32, Path, description = "Database ID of note to get"),
    ),
    responses(
        (status = 200, description = "Successfully got note", body = models::notes::Note),
        (status = 404, description = "Note not found")
    )
)]
#[axum::debug_handler]
pub async fn get_note(
    State(pool): State<utils::Pool>,
    Path(id): Path<i32>,
) -> Result<Json<models::notes::Note>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let res = schema::notes::table
        .select(models::notes::Note::as_select())
        .filter(schema::notes::id.eq(id))
        .first(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

#[utoipa::path(
    get,
    path = "",
    params(models::notes::NoteQueryParams),
    responses(
        (status = 200, description = "Successfully got notes", body = [models::notes::Note]),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_notes(
    State(state): State<state::ToiState>,
    Query(params): Query<models::notes::NoteQueryParams>,
) -> Result<Json<Vec<models::notes::Note>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let mut query = schema::notes::table
        .select(models::notes::Note::as_select())
        .into_boxed();

    // Filter notes similar to a query.
    if let Some(note_similarity_search_params) = params.similarity_search_params {
        let embedding_request = models::client::EmbeddingRequest {
            input: note_similarity_search_params.query,
        };
        let embedding = state.client.embed(embedding_request).await?;
        query = query.filter(
            schema::notes::embedding
                .cosine_distance(embedding.clone())
                .le(note_similarity_search_params.distance_threshold),
        );

        // Sort by relevance.
        if params.order_by == Some(utils::OrderBy::Relevance) {
            query = query.order(schema::notes::embedding.l2_distance(embedding));
        }
    }

    // Filter notes created on or after date.
    if let Some(from) = params.from {
        query = query.filter(schema::notes::created_at.ge(from));
    }

    // Filter notes created on or before date.
    if let Some(to) = params.to {
        query = query.filter(schema::notes::created_at.le(to));
    }

    match params.order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::notes::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::notes::created_at.desc()),
        _ => {}
    }

    // Limit number of notes returned.
    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    let res = query.load(&mut conn).await.map_err(utils::internal_error)?;
    Ok(Json(res))
}
