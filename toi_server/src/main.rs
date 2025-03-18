use std::fs::File;

use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

mod client;
mod models;
mod routes;
mod schema;
mod state;
mod utils;

#[derive(serde::Deserialize)]
struct ToiConfig {
    #[serde(default = "utils::default_server_binding_addr")]
    binding_addr: String,
    embedding_api_config: models::client::HttpClientConfig,
    generation_api_config: models::client::HttpClientConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    let config_path = dotenvy::var("TOI_CONFIG_PATH")?;
    let config_file = File::open(config_path)?;
    let config: ToiConfig = serde_json::from_reader(config_file)?;
    let ToiConfig {
        binding_addr,
        embedding_api_config,
        generation_api_config,
    } = config;

    let client = client::Client::new(embedding_api_config, generation_api_config)?;
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_connection_url);
    let pool = bb8::Pool::builder().build(manager).await?;
    let state = state::ToiState { client, pool };

    let (router, api) = OpenApiRouter::new()
        .nest("/datetime", routes::datetime::router())
        .nest("/notes", routes::notes::router(state))
        .split_for_parts();
    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/docs/openapi.json", api));

    let listener = TcpListener::bind(binding_addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
