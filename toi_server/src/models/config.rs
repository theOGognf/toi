use crate::{models::client::HttpClientConfig, utils};
use serde::Deserialize;
use std::default;
use std::fmt;

#[derive(Clone, Deserialize)]
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

#[derive(Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "utils::default_server_binding_addr")]
    pub bind_addr: String,
    #[serde(deserialize_with = "utils::deserialize_with_envsubst")]
    pub user_agent: UserAgent,
    #[serde(default = "utils::default_distance_threshold")]
    pub distance_threshold: f64,
    #[serde(default = "utils::default_similarity_threshold")]
    pub similarity_threshold: f64,
}

#[derive(Deserialize)]
pub struct ToiConfig {
    pub server: ServerConfig,
    pub embedding: HttpClientConfig,
    pub generation: HttpClientConfig,
    pub reranking: HttpClientConfig,
}
