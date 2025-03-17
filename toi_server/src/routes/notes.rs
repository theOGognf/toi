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
use pgvector::{Vector, VectorExpressionMethods};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{models, schema, utils};

pub fn router(state: utils::Pool) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_note, search_notes, get_note))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "",
    responses(
        (status = 201, description = "Successfully added a note", body = models::notes::Note)
    )
)]
#[axum::debug_handler]
pub async fn add_note(
    State(pool): State<utils::Pool>,
    Json(new_note_request): Json<models::notes::NewNoteRequest>,
) -> Result<Json<models::notes::Note>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let new_note = models::notes::NewNote {
        content: new_note_request.content,
        embedding: vec![].into(),
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
        (status = 200, description = "Successfully searched notes", body = [models::notes::Note])
    )
)]
#[axum::debug_handler]
pub async fn search_notes(
    State(pool): State<utils::Pool>,
    Query(params): Query<models::notes::NoteQueryParams>,
) -> Result<Json<Vec<models::notes::Note>>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let mut query = schema::notes::table
        .select(models::notes::Note::as_select())
        .into_boxed();

    // Find notes similar to a query.
    if let Some(note_similarity_search_params) = params.similarity_search_params {
        todo!("Need to actually compute embedding for the query content.");
        let embedding = Vector::from(vec![3.0]);
        query = query
            .filter(
                schema::notes::embedding
                    .cosine_distance(embedding.clone())
                    .le(note_similarity_search_params.threshold),
            )
            .order(schema::notes::embedding.l2_distance(embedding))
            .limit(note_similarity_search_params.limit)
            .limit(5)
    }

    // Get notes created on or after date.
    if let Some(from) = params.from {
        query = query.filter(schema::notes::created_at.ge(from));
    }

    // Get notes created on or before date.
    if let Some(to) = params.to {
        query = query.filter(schema::notes::created_at.le(to));
    }

    let res = query.load(&mut conn).await.map_err(utils::internal_error)?;
    Ok(Json(res))
}
