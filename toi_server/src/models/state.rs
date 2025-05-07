use crate::{client::ModelClient, models::config::ServerConfig, utils};
use axum::extract::FromRef;

#[derive(Clone)]
pub struct ToiState {
    pub server_config: ServerConfig,
    pub model_client: ModelClient,
    pub pool: utils::Pool,
}

impl FromRef<ToiState> for ModelClient {
    fn from_ref(state: &ToiState) -> ModelClient {
        state.model_client.clone()
    }
}

impl FromRef<ToiState> for utils::Pool {
    fn from_ref(state: &ToiState) -> utils::Pool {
        state.pool.clone()
    }
}

impl FromRef<ToiState> for ServerConfig {
    fn from_ref(state: &ToiState) -> ServerConfig {
        state.server_config.clone()
    }
}
