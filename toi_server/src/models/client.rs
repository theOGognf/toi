use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_query::DeserializeQuery;
use std::collections::HashMap;
use toi::Message;

#[derive(Serialize)]
pub struct EmbeddingRequest {
    pub input: String,
}

#[derive(Deserialize, DeserializeQuery)]
pub struct EmbeddingResponse {
    #[query(".data.[0].embedding")]
    pub embedding: Vec<f32>,
}

#[derive(Serialize)]
pub struct StreamOptions {
    stream: bool,
    include_usage: bool,
}

#[derive(Serialize)]
pub struct StreamingGenerationRequest {
    pub messages: Vec<Message>,
    stream_options: StreamOptions,
}

impl StreamingGenerationRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            stream_options: StreamOptions {
                stream: true,
                include_usage: true,
            },
        }
    }
}

#[derive(Serialize, serde_query::Deserialize)]
pub struct GenerationResponse {
    #[query(".choices.[0].message.content")]
    pub content: String,
}

pub enum ModelClientError {
    ApiConnection,
    DefaultJson,
    RequestJson,
    ResponseJson,
}

impl ModelClientError {
    pub fn into_response(self, err: &str) -> (StatusCode, String) {
        match self {
            Self::ApiConnection => (
                StatusCode::BAD_GATEWAY,
                format!("connection error when getting response: {err}"),
            ),
            Self::DefaultJson => (
                StatusCode::BAD_REQUEST,
                format!("couldn't serialize default JSON: {err}"),
            ),
            Self::RequestJson => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("couldn't serialize request: {err}"),
            ),
            Self::ResponseJson => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("couldn't deserialize response: {err}"),
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
