use axum::http::StatusCode;
use bon::Builder;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
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

#[derive(Serialize, Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Debug)]
pub enum LoadConfigError {
    Deserialization(serde_json::Error),
    EnvVarSubstitution(envsubst::Error),
}

impl fmt::Display for LoadConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Self::Deserialization(err) => err.to_string(),
            Self::EnvVarSubstitution(err) => err.to_string(),
        };
        write!(f, "{repr}")
    }
}

fn substitute<'de, D>(value: &mut Value, vars: &HashMap<String, String>) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    match value {
        Value::String(s) => {
            *s = envsubst::substitute(s.clone(), vars)
                .map_err(LoadConfigError::EnvVarSubstitution)
                .map_err(serde::de::Error::custom)?;
        }
        Value::Array(arr) => {
            for item in arr {
                substitute::<D>(item, vars)?;
            }
        }
        Value::Object(map) => {
            for (_, val) in map {
                substitute::<D>(val, vars)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn deserialize_with_envsubst<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let json_str: &str = Deserialize::deserialize(deserializer)?;
    let mut value: Value = serde_json::from_str(json_str)
        .map_err(LoadConfigError::Deserialization)
        .map_err(serde::de::Error::custom)?;
    let vars: HashMap<String, String> = std::env::vars().collect();
    substitute::<D>(&mut value, &vars)?;
    T::deserialize(value)
        .map_err(LoadConfigError::Deserialization)
        .map_err(serde::de::Error::custom)
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct HttpClientConfig {
    pub base_url: String,
    #[serde(deserialize_with = "deserialize_with_envsubst")]
    pub headers: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_with_envsubst")]
    pub params: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_with_envsubst")]
    pub json: HashMap<String, String>,
}
