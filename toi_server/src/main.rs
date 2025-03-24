use std::fs::File;

use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

mod client;
mod models;
mod routes;
mod schema;
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
    // All configuration comes from environment variables and a required
    // config file.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    let config_path = dotenvy::var("TOI_CONFIG_PATH")?;
    let config_file = File::open(config_path)?;
    let config: ToiConfig = serde_json::from_reader(config_file)?;
    let ToiConfig {
        binding_addr,
        embedding_api_config,
        generation_api_config,
    } = config;

    // Shared state components. A client is used for interacting with supporting
    // API services, while a pool is used for interacting with the database.
    let client = client::Client::new(embedding_api_config, generation_api_config)?;
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_connection_url);
    let pool = bb8::Pool::builder().build(manager).await?;
    // Build state with empty spec first since only the assistant endpoint uses
    // the OpenAPI spec.
    let mut state = models::state::ToiState {
        openapi_spec: "".to_string(),
        client,
        pool,
    };

    // Define base router and OpenAPI spec used for building the system prompt
    // for the main assistant endpoint.
    let openapi_router = OpenApiRouter::new()
        .nest("/datetime", routes::datetime::router())
        .nest("/notes", routes::notes::router(state.clone()));
    let openapi_spec = openapi_router.get_openapi().to_pretty_json()?;

    // Add the main assistant endpoint to the router so it can be included in
    // the docs, but excluded from its own system prompt. Then continue building
    // the API routes.
    state.openapi_spec = openapi_spec;
    let openapi_router = openapi_router.nest("/assist", routes::assist::router(state));
    let (router, api) = openapi_router.split_for_parts();
    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/docs/openapi.json", api));

    let listener = TcpListener::bind(binding_addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
