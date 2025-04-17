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

use crate::{
    models::{
        client::EmbeddingRequest,
        notes::{NewNote, NewNoteRequest, Note, NoteQueryParams},
        state::ToiState,
    },
    schema, utils,
};

pub fn router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_note, delete_note, get_note))
        .routes(routes!(delete_matching_notes, get_matching_notes))
        .with_state(state)
}

/// Add a note.
#[utoipa::path(
    post,
    path = "",
    request_body = NewNoteRequest,
    responses(
        (status = 201, description = "Successfully added a note", body = Note),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_note(
    State(state): State<ToiState>,
    Json(new_note_request): Json<NewNoteRequest>,
) -> Result<Json<Note>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let embedding_request = EmbeddingRequest {
        input: new_note_request.content.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let new_note = NewNote {
        content: new_note_request.content,
        embedding,
    };
    let res = diesel::insert_into(schema::notes::table)
        .values(new_note)
        .returning(Note::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete a note by its database-generated ID.
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
) -> Result<Json<Note>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let res = diesel::delete(schema::notes::table)
        .filter(schema::notes::id.eq(id))
        .returning(Note::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete notes that match search criteria.
///
/// Useful for deleting notes in bulk.
#[utoipa::path(
    delete,
    path = "/search",
    params(NoteQueryParams),
    responses(
        (status = 200, description = "Successfully deleted notes", body = [Note]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_notes(
    State(state): State<ToiState>,
    Query(params): Query<NoteQueryParams>,
) -> Result<Json<Vec<Note>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let mut query = schema::notes::table.select(schema::notes::id).into_boxed();

    // Filter notes similar to a query.
    if let Some(note_similarity_search_params) = params.similarity_search_params {
        let embedding_request = EmbeddingRequest {
            input: note_similarity_search_params.query,
        };
        let embedding = state.model_client.embed(embedding_request).await?;
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
        .returning(Note::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::internal_error)?;
    Ok(Json(res))
}

/// Get a note by its database-generated ID.
#[utoipa::path(
    get,
    path = "/{id}",
    params(
        ("id" = i32, Path, description = "Database ID of note to get"),
    ),
    responses(
        (status = 200, description = "Successfully got note", body = Note),
        (status = 404, description = "Note not found")
    )
)]
#[axum::debug_handler]
pub async fn get_note(
    State(pool): State<utils::Pool>,
    Path(id): Path<i32>,
) -> Result<Json<Note>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let res = schema::notes::table
        .select(Note::as_select())
        .filter(schema::notes::id.eq(id))
        .first(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Get notes that match search criteria.
///
/// Useful for getting notes in bulk.
#[utoipa::path(
    get,
    path = "/search",
    params(NoteQueryParams),
    responses(
        (status = 200, description = "Successfully got notes", body = [Note]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_notes(
    State(state): State<ToiState>,
    Query(params): Query<NoteQueryParams>,
) -> Result<Json<Vec<Note>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let mut query = schema::notes::table.select(Note::as_select()).into_boxed();

    // Filter notes similar to a query.
    if let Some(note_similarity_search_params) = params.similarity_search_params {
        let embedding_request = EmbeddingRequest {
            input: note_similarity_search_params.query,
        };
        let embedding = state.model_client.embed(embedding_request).await?;
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
