use std::fs::File;

use ctrlc::set_handler;
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use tracing::info;

mod client;
pub mod models;
pub mod routes;
pub mod schema;
mod utils;

pub async fn init(
    db_connection_url: String,
) -> Result<models::state::ToiState, Box<dyn std::error::Error>> {
    // Catching signals for exit.
    set_handler(|| std::process::exit(0))?;

    // All configuration comes from a required config file.
    let config_path = dotenvy::var("TOI_CONFIG_PATH")?;
    let config_file = File::open(config_path)?;
    let config: models::config::ToiConfig = serde_json::from_reader(config_file)?;
    info!("initializing with {config:?}");
    let models::config::ToiConfig {
        server: server_config,
        embedding: embedding_api_config,
        generation: generation_api_config,
        reranking: reranking_api_config,
    } = config;

    // Shared state components. A client is used for interacting with supporting
    // API services, while a pool is used for interacting with the database.
    let model_client = client::ModelClient::new(
        embedding_api_config,
        generation_api_config,
        reranking_api_config,
    )?;
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_connection_url);
    let pool = bb8::Pool::builder().build(manager).await?;

    // Build state with empty spec first since only the assistant endpoint uses
    // the OpenAPI spec.
    let state = models::state::ToiState {
        server_config,
        model_client,
        pool,
    };
    Ok(state)
}
