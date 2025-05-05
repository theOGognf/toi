use axum::extract::FromRef;

use crate::{client::ModelClient, utils};

#[derive(Clone)]
pub struct ToiState {
    pub binding_addr: String,
    pub model_client: ModelClient,
    pub pool: utils::Pool,
    pub user_agent: String,
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
