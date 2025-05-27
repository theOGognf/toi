use diesel::{Connection, PgConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
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
            "/banking/accounts",
            toi_server::routes::accounts::accounts_router(state.clone()).nest(
                "/transactions",
                toi_server::routes::transactions::bank_account_transactions_router(state.clone()),
            ),
        )
        .nest(
            "/banking/transactions",
            toi_server::routes::transactions::transactions_router(state.clone()),
        )
        .nest(
            "/contacts",
            toi_server::routes::contacts::contacts_router(state.clone()),
        )
        .nest("/datetime", toi_server::routes::datetime::datetime_router())
        .nest(
            "/events",
            toi_server::routes::events::events_router(state.clone()).nest(
                "/attendees",
                toi_server::routes::attendees::attendees_router(state.clone()),
            ),
        )
        .nest(
            "/news",
            toi_server::routes::news::news_router(state.clone()).await?,
        )
        .nest(
            "/notes",
            toi_server::routes::notes::notes_router(state.clone()),
        )
        .nest(
            "/recipes",
            toi_server::routes::recipes::recipes_router(state.clone()),
        )
        .nest(
            "/tags",
            toi_server::routes::tags::tags_router(state.clone()),
        )
        .nest(
            "/todos",
            toi_server::routes::todos::todos_router(state.clone()),
        )
        .nest(
            "/weather",
            toi_server::routes::weather::weather_router(state.clone()),
        );
    let openapi = openapi_router.get_openapi_mut();

    // Add the main /chat endpoint to the router so it can be included in
    // the docs, but excluded from its own system prompt. Then continue building
    // the API router.
    let chat_router = toi_server::routes::chat::chat_router(openapi, state.clone()).await?;
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
