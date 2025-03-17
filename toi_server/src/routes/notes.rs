use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use diesel::{
    ExpressionMethods,
    prelude::{QueryDsl, SelectableHelper},
};
use diesel_async::RunQueryDsl;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{models, schema, utils};

pub fn router(state: utils::Pool) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_note, filter_notes, get_note))
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
    path = "", 
    responses(
        (status = 200, description = "Successfully filtered notes", body = [models::notes::Note])
    )
)]
#[axum::debug_handler]
pub async fn filter_notes(
    State(pool): State<utils::Pool>,
) -> Result<Json<Vec<models::notes::Note>>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(utils::internal_error)?;
    let res = schema::notes::table
        .select(models::notes::Note::as_select())
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
