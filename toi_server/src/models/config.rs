use crate::{client::ModelClient, models::client::HttpClientConfig, utils};
use axum::extract::FromRef;
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
    pub binding_addr: String,
    #[serde(deserialize_with = "utils::deserialize_with_envsubst")]
    pub user_agent: UserAgent,
}

#[derive(Deserialize)]
pub struct ToiConfig {
    pub server_config: ServerConfig,
    pub embedding_api_config: HttpClientConfig,
    pub generation_api_config: HttpClientConfig,
    pub reranking_api_config: HttpClientConfig,
}
