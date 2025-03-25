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

#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    #[serde(skip)]
    System,
    Assistant,
    User,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Deserialize, Serialize)]
pub struct GenerationRequest {
    pub messages: Vec<Message>,
}

#[derive(Deserialize, Serialize, DeserializeQuery)]
pub struct GenerationResponse {
    #[query(".choices.[0].message.content")]
    pub content: String,
}

pub enum ClientError {
    ApiConnection,
    DefaultJson,
    RequestJson,
    ResponseJson,
}

impl ClientError {
    pub fn to_response(self, url: &str, original_err: &str) -> (StatusCode, String) {
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
