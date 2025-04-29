use axum::http::StatusCode;
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

#[derive(PartialEq, Deserialize, Serialize, JsonSchema, ToSchema)]
pub enum OrderBy {
    Oldest,
    Newest,
    Relevance,
}

pub fn default_distance_threshold() -> f64 {
    0.85
}

pub fn default_server_binding_addr() -> String {
    "127.0.0.1:6969".to_string()
}

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
