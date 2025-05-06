use crate::{client::ModelClient, utils};
use axum::extract::FromRef;
use serde::Deserialize;
use std::fmt;

#[derive(Clone, Deserialize)]
pub struct UserAgent(String);

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
pub struct ToiState {
    pub binding_addr: String,
    pub model_client: ModelClient,
    pub pool: utils::Pool,
    pub user_agent: UserAgent,
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

impl FromRef<ToiState> for UserAgent {
    fn from_ref(state: &ToiState) -> UserAgent {
        state.user_agent.clone()
    }
}
