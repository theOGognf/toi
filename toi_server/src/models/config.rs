use crate::{models::client::HttpClientConfig, utils};
use serde::Deserialize;
use std::default;
use std::fmt;

#[derive(Clone, Debug, Deserialize)]
pub struct UserAgent(String);

impl default::Default for UserAgent {
    fn default() -> Self {
        Self("https://github.com/theOGognf/toi".to_string())
    }
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn default_distance_threshold() -> f64 {
    0.85
}

fn default_similarity_threshold() -> f64 {
    0.45
}

fn default_server_bind_addr() -> String {
    "127.0.0.1:6969".to_string()
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server_bind_addr")]
    pub bind_addr: String,
    #[serde(deserialize_with = "utils::deserialize_with_envsubst")]
    pub user_agent: UserAgent,
    #[serde(default = "default_distance_threshold")]
    pub distance_threshold: f64,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,
}

#[derive(Debug, Deserialize)]
pub struct ToiConfig {
    pub server: ServerConfig,
    pub embedding: HttpClientConfig,
    pub generation: HttpClientConfig,
    pub reranking: HttpClientConfig,
}
