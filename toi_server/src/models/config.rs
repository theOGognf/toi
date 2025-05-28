use crate::{models::client::HttpClientConfig, utils};
use serde::Deserialize;
use std::net::SocketAddr;

fn default_bind_addr() -> SocketAddr {
    "127.0.0.1:6969"
        .parse()
        .expect("default bind address should be valid")
}

fn default_distance_threshold() -> f64 {
    0.75
}

fn default_similarity_threshold() -> f64 {
    0.50
}

fn default_user_agent() -> String {
    "https://github.com/theOGognf/toi".to_string()
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(
        default = "default_bind_addr",
        deserialize_with = "utils::deserialize_socket_addr"
    )]
    pub bind_addr: SocketAddr,
    #[serde(
        default = "default_user_agent",
        deserialize_with = "utils::deserialize_with_envsubst"
    )]
    pub user_agent: String,
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
