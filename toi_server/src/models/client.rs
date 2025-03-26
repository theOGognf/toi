use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_query::DeserializeQuery;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct EmbeddingRequest {
    pub input: String,
}

#[derive(Deserialize, DeserializeQuery)]
pub struct EmbeddingResponse {
    #[query(".data.[0].embedding")]
    pub embedding: Vec<f32>,
}

pub enum ModelClientError {
    ApiConnection,
    DefaultJson,
    RequestJson,
    ResponseJson,
}

impl ModelClientError {
    pub fn into_response(self, url: &str, original_err: &str) -> (StatusCode, String) {
        match self {
            Self::ApiConnection => (
                StatusCode::BAD_GATEWAY,
                format!(
                    "connection error when getting response from {url} resultin in '{original_err}'"
                ),
            ),
            Self::DefaultJson => (
                StatusCode::BAD_REQUEST,
                format!("couldn't serialize default JSON for {url} resulting in '{original_err}'"),
            ),
            Self::RequestJson => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("couldn't serialize request for {url} resulting in '{original_err}'"),
            ),
            Self::ResponseJson => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("couldn't deserialize response from {url} resulting in '{original_err}'"),
            ),
        }
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct HttpClientConfig {
    pub base_url: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub json: HashMap<String, String>,
}
