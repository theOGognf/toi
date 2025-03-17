use axum::http::StatusCode;
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};

pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

/// Map Diesel errors into a specific response.
pub fn diesel_error(err: diesel::result::Error) -> (StatusCode, String) {
    match err {
        diesel::result::Error::NotFound => (StatusCode::NOT_FOUND, err.to_string()),
        _ => internal_error(err),
    }
}

/// Map any error into a `500 Internal Server Error` response.
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
