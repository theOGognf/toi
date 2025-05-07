use crate::utils;
use axum::http::StatusCode;
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use toi::Message;

#[derive(Builder, Clone, Deserialize)]
pub struct EmbeddingPromptTemplate {
    pub instruction_prefix: Option<String>,
    pub query_prefix: Option<String>,
}

impl EmbeddingPromptTemplate {
    #[must_use]
    pub fn apply(&self, query: &str) -> String {
        match (&self.instruction_prefix, &self.query_prefix) {
            (Some(instruction_prefix), Some(query_prefix)) => {
                format!("{instruction_prefix}\n{query_prefix}{query}")
            }
            (Some(instruction_prefix), None) => format!("{instruction_prefix}\n{query}"),
            (None, Some(query_prefix)) => format!("{query_prefix}{query}"),
            (None, None) => query.to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct EmbeddingRequest {
    pub input: String,
}

#[derive(Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
}

#[derive(Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

#[derive(Serialize)]
pub struct RerankRequest {
    pub query: String,
    pub documents: Vec<String>,
}

#[derive(Deserialize)]
pub struct RerankDocument {
    pub text: String,
}

#[derive(Deserialize)]
pub struct RerankResult {
    pub index: usize,
    pub document: RerankDocument,
    pub relevance_score: f64,
}

#[derive(Deserialize)]
pub struct RerankResponse {
    pub results: Vec<RerankResult>,
}

#[derive(Serialize)]
pub struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
pub struct StreamingGenerationRequest {
    pub messages: Vec<Message>,
    stream: bool,
    stream_options: StreamOptions,
}

impl StreamingGenerationRequest {
    #[must_use]
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            stream: true,
            stream_options: StreamOptions {
                include_usage: true,
            },
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Deserialize, Serialize)]
pub struct GenerationResponse {
    pub choices: Vec<Choice>,
}

pub enum ApiClientError {
    ApiConnection,
    DefaultJson,
    EmptyResponse,
    RequestJson,
    ResponseJson,
}

impl ApiClientError {
    #[must_use]
    pub fn into_response<T: fmt::Debug>(self, err: &T) -> (StatusCode, String) {
        match self {
            Self::ApiConnection => (
                StatusCode::BAD_GATEWAY,
                format!("connection error when getting response: {err:?}"),
            ),
            Self::DefaultJson => (
                StatusCode::BAD_REQUEST,
                format!("couldn't serialize default JSON: {err:?}"),
            ),
            Self::EmptyResponse => (StatusCode::NOT_FOUND, format!("item not found: {err:?}")),
            Self::RequestJson => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("couldn't serialize request: {err:?}"),
            ),
            Self::ResponseJson => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("couldn't deserialize response: {err:?}"),
            ),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct HttpClientConfig {
    pub base_url: String,
    #[serde(deserialize_with = "utils::deserialize_with_envsubst")]
    pub headers: HashMap<String, String>,
    #[serde(deserialize_with = "utils::deserialize_with_envsubst")]
    pub params: HashMap<String, String>,
    #[serde(deserialize_with = "utils::deserialize_with_envsubst")]
    pub json: HashMap<String, String>,
}
