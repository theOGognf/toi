use std::fs::File;

use ctrlc::set_handler;
use diesel::{Connection, PgConnection};
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use serde::Deserialize;
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

mod client;
mod models;
mod routes;
mod schema;
mod utils;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(OpenApi)]
#[openapi(info(
    title = "Personal Assistant Server",
    description = "Endpoints to perform actions like a personal assistant would"
))]
struct ApiDoc;

#[derive(Deserialize)]
struct ToiConfig {
    #[serde(default = "utils::default_server_binding_addr")]
    binding_addr: String,
    embedding_api_config: models::client::HttpClientConfig,
    generation_api_config: models::client::HttpClientConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let mut state = models::state::ToiState {
        openapi_spec: "".to_string(),
        model_client,
        pool,
    };

    // Define base router and OpenAPI spec used for building the system prompt
    // for the main assistant endpoint.
    let openapi_router = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/datetime", routes::datetime::router())
        .nest("/notes", routes::notes::router(state.clone()));
    let openapi = openapi_router.get_openapi();

    // Go through and embed all OpenAPI path specs so they can be used as
    // context for generating HTTP requests within the "/chat" endpoint.
    let mut conn = state.pool.get().await?;
    for (path, item) in openapi.paths.paths.iter() {
        // Make a pretty JSON for embedding and storing the spec.
        let item = serde_json::to_value(item)?;
        let spec = json!(
            {
                path: item
            }
        );

        // Embed and store the spec.
        let embedding_request = models::client::EmbeddingRequest {
            input: serde_json::to_string_pretty(&spec)?,
        };
        let embedding = state
            .model_client
            .embed(embedding_request)
            .await
            .map_err(|(_, err)| err)?;
        let new_openapi_path = models::openapi::NewOpenApiPath { spec, embedding };
        diesel::insert_into(schema::openapi::table)
            .values(new_openapi_path)
            .get_result(&mut conn)
            .await?;
    }

    // Add the main assistant endpoint to the router so it can be included in
    // the docs, but excluded from its own system prompt. Then continue building
    // the API routes.
    state.openapi_spec = openapi.to_pretty_json()?;
    let openapi_router = openapi_router.nest("/chat", routes::chat::router(state));
    let (router, api) = openapi_router.split_for_parts();
    let router = router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new())
                .on_response(DefaultOnResponse::new()),
        );

    let listener = TcpListener::bind(binding_addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
