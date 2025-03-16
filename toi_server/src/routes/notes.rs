use axum::{extract::State, http::StatusCode, response::Json};
use diesel::prelude::{QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{models, schema, utils};

pub fn router(state: utils::Pool) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(create_note, list_notes))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "",
    responses(
        (status = 201, description = "Successfully created note", body = models::notes::Note)
    )
)]
#[axum::debug_handler]
pub async fn create_note(
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
        .map_err(utils::internal_error)?;
    Ok(Json(res))
}

#[utoipa::path(
    get,
    path = "", 
    responses(
        (status = 200, description = "Successfully listed notes", body = [models::notes::Note])
    )
)]
#[axum::debug_handler]
pub async fn list_notes(
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
