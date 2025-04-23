use diesel::{Connection, PgConnection};
use diesel_async::RunQueryDsl;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(OpenApi)]
#[openapi(info(
    title = "Personal Assistant Server",
    description = "Endpoints to perform actions like a personal assistant would"
))]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // An explicit database URL is required for setup.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;

    // Get a connection and manually run migrations at startup just in case
    // to ensure the database is ready to go.
    let mut conn = PgConnection::establish(&db_connection_url)?;
    conn.run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    // Initialize the server state and extract the server binding address.
    let (binding_addr, mut state) = toi_server::init(db_connection_url).await?;

    // Define base router and OpenAPI spec used for building the system prompt
    // for the main assistant endpoint.
    let openapi_router = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/datetime", toi_server::routes::datetime::router())
        .nest("/notes", toi_server::routes::notes::router(state.clone()));
    let openapi = openapi_router.get_openapi();

    // Go through and embed all OpenAPI path specs so they can be used as
    // context for generating HTTP requests within the "/chat" endpoint.
    let pool = state.pool.clone();
    let mut conn = pool.get().await?;
    for (path, item) in openapi.paths.paths.iter() {
        for (method, op) in [
            ("delete", &item.delete),
            ("get", &item.get),
            ("post", &item.post),
            ("put", &item.put),
        ] {
            if let Some(op) = op {
                // Make a pretty JSON for storing the spec.
                let method = method.to_string();
                let item = serde_json::to_value(item)?;
                let spec = json!(
                    {
                        path: {
                            method: item
                        }
                    }
                );

                // Make the description from the operation's summary and description.
                // This is what is used for semantic search rather than the spec
                // itself so it's more likely to match with user queries.
                let description = [op.summary.clone(), op.description.clone()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<String>>()
                    .join("\n\n");

                // Embed the endpoint's description.
                let embedding_request = toi_server::models::client::EmbeddingRequest {
                    input: serde_json::to_string_pretty(&description)?,
                };
                let embedding = state
                    .model_client
                    .embed(embedding_request)
                    .await
                    .map_err(|(_, err)| err)?;

                // Store all the details.
                let new_openapi_path =
                    toi_server::models::openapi::NewOpenApiPath { spec, embedding };
                diesel::insert_into(toi_server::schema::openapi::table)
                    .values(new_openapi_path)
                    .execute(&mut conn)
                    .await?;
            }
        }
    }

    // Add the main assistant endpoint to the router so it can be included in
    // the docs, but excluded from its own system prompt. Then continue building
    // the API routes.
    state.openapi_spec = openapi.to_pretty_json()?;
    let openapi_router = openapi_router.nest("/chat", toi_server::routes::chat::router(state));
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
