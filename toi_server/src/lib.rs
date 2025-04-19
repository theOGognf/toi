use std::fs::File;

use ctrlc::set_handler;
use diesel::{Connection, PgConnection};
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use serde::Deserialize;
use tracing_subscriber::EnvFilter;

mod client;
pub mod models;
pub mod routes;
pub mod schema;
mod utils;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(Deserialize)]
pub struct ToiConfig {
    #[serde(default = "utils::default_server_binding_addr")]
    binding_addr: String,
    embedding_api_config: models::client::HttpClientConfig,
    generation_api_config: models::client::HttpClientConfig,
}

type BindingAddress = String;

pub async fn init() -> Result<(BindingAddress, models::state::ToiState), Box<dyn std::error::Error>>
{
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    // Catching signals for exit.
    set_handler(|| std::process::exit(0))?;

    // All configuration comes from environment variables and a required
    // config file.
    let db_connection_url = dotenvy::var("DB_URL")?;
    let config_path = dotenvy::var("TOI_CONFIG_PATH")?;
    let config_file = File::open(config_path)?;
    let config: ToiConfig = serde_json::from_reader(config_file)?;
    let ToiConfig {
        binding_addr,
        embedding_api_config,
        generation_api_config,
    } = config;

    // Get a connection and manually run migrations at startup just in case
    // to ensure the database is ready to go.
    let mut conn = PgConnection::establish(&db_connection_url)?;
    conn.run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    // Shared state components. A client is used for interacting with supporting
    // API services, while a pool is used for interacting with the database.
    let model_client = client::ModelClient::new(embedding_api_config, generation_api_config)?;
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_connection_url);
    let pool = bb8::Pool::builder().build(manager).await?;

    // Build state with empty spec first since only the assistant endpoint uses
    // the OpenAPI spec.
    let state = models::state::ToiState {
        openapi_spec: "".to_string(),
        model_client,
        pool,
    };
    Ok((binding_addr, state))
}
