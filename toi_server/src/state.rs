use axum::extract::FromRef;

use crate::{client, utils};

#[derive(Clone)]
pub struct ToiState {
    pub client: client::Client,
    pub pool: utils::Pool,
}

impl FromRef<ToiState> for client::Client {
    fn from_ref(state: &ToiState) -> client::Client {
        state.client.clone()
    }
}

impl FromRef<ToiState> for utils::Pool {
    fn from_ref(state: &ToiState) -> utils::Pool {
        state.pool.clone()
    }
}
