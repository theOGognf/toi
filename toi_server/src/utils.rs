use axum::http::StatusCode;
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};

pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
