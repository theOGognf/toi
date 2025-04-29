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
    0.2
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

/// Extract the first JSON object from a string.
pub fn extract_json(s: &str) -> Result<&str, &str> {
    let mut depth = 0;
    let mut start_idx = None;
    let mut end_idx = None;
    for (c_idx, c) in s.char_indices() {
        match (start_idx, c) {
            (None, '{') => {
                start_idx = Some(c_idx);
                depth += 1;
            }
            (Some(_), '{') => {
                depth += 1;
            }
            (Some(_), '}') => {
                depth -= 1;
                if depth == 0 {
                    end_idx = Some(c_idx + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    match (start_idx, end_idx) {
        (Some(s_idx), Some(e_idx)) => Ok(&s[s_idx..e_idx]),
        _ => Err("no JSON object found in string"),
    }
}

/// Map any error into a `500 Internal Server Error` response.
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
