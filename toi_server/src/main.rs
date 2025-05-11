use diesel::{Connection, PgConnection, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(OpenApi)]
#[openapi(info(
    title = "Personal Assistant Server",
    description = "Endpoints to perform actions like a personal assistant would"
))]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // An explicit database URL is required for setup.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    info!("connecting to {db_connection_url}");
    let mut conn = PgConnection::establish(&db_connection_url)?;
    info!("running migrations");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    // Initialize the server state and extract the server binding address.
    info!("initializing server state");
    let state = toi_server::init(db_connection_url).await?;

    // Define base router and OpenAPI spec used for building the system prompt
    // for the main assistant endpoint.
    let mut openapi_router = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(
            "/contacts",
            toi_server::routes::contacts::router(state.clone()),
        )
        .nest("/datetime", toi_server::routes::datetime::router())
        .nest(
            "/events",
            toi_server::routes::events::router(state.clone()).nest(
                "/participants",
                toi_server::routes::participants::router(state.clone()),
            ),
        )
        .nest("/notes", toi_server::routes::notes::router(state.clone()))
        .nest("/todos", toi_server::routes::todos::router(state.clone()))
        .nest(
            "/weather",
            toi_server::routes::weather::router(state.clone()),
        );
    let openapi = openapi_router.get_openapi_mut();

    // Add the main /chat endpoint to the router so it can be included in
    // the docs, but excluded from its own system prompt. Then continue building
    // the API router.
    let chat_router = toi_server::routes::chat::router(openapi, state.clone()).await?;
    let openapi_router = openapi_router.nest("/chat", chat_router);
    let (router, api) = openapi_router.split_for_parts();
    let router = router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(TraceLayer::new_for_http());

    info!("serving at {}", state.server_config.bind_addr);
    let listener = TcpListener::bind(state.server_config.bind_addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
