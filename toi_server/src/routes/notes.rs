use axum::{extract::State, http::StatusCode, response::Json};

use diesel::prelude::{QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;

use crate::{models, schema, utils};

// we can extract the connection pool with `State`
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
