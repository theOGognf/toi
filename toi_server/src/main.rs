use diesel::{Connection, PgConnection, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
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
        .nest("/todos", toi_server::routes::todos::router(state.clone()));
    let openapi = openapi_router.get_openapi_mut();

    // Go through and embed all OpenAPI path specs so they can be used as
    // context for generating HTTP requests within the /chat endpoint.
    // Start by deleting all the pre-existing OpenAPI path specs just in
    // case.
    info!("preparing OpenAPI endpoints for automation");
    diesel::delete(toi_server::schema::openapi::table).execute(&mut conn)?;
    for (path, item) in &mut openapi.paths.paths {
        for (method, op) in [
            ("DELETE", &mut item.delete),
            ("GET", &mut item.get),
            ("POST", &mut item.post),
            ("PUT", &mut item.put),
        ] {
            if let Some(op) = op {
                // This is what is used for semantic search rather than the spec
                // itself so it's more likely to match with user queries.
                let description = [op.summary.clone(), op.description.clone()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<String>>()
                    .join("\n\n");

                if description.is_empty() {
                    warn!(
                        "skipping uri={path} method={method} due to missing context from doc string"
                    );
                    continue;
                }

                // Get params and body from OpenAPI extensions.
                let (params, body) = match op.extensions {
                    Some(ref mut extensions) => (
                        extensions.remove("x-json-schema-params"),
                        extensions.remove("x-json-schema-body"),
                    ),
                    None => (None, None),
                };

                // Make sure the JSON schema params match the params in the OpenAPI spec.
                match (&op.parameters, &params) {
                    (Some(_), Some(_)) | (None, None) => {}
                    (Some(_), None) => {
                        warn!(
                            "skipping uri={path} method={method} due to missing JSON schema parameters"
                        );
                        continue;
                    }
                    (None, Some(_)) => {
                        warn!(
                            "skipping uri={path} method={method} due to extra JSON schema parameters"
                        );
                        continue;
                    }
                }

                // Make sure the JSON schema params match the params in the OpenAPI spec.
                match (&op.request_body, &body) {
                    (Some(_), Some(_)) | (None, None) => {}
                    (Some(_), None) => {
                        warn!(
                            "skipping uri={path} method={method} due to missing JSON schema body"
                        );
                        continue;
                    }
                    (None, Some(_)) => {
                        warn!("skipping uri={path} method={method} due to extra JSON schema body");
                        continue;
                    }
                }

                // Embed the endpoint's query.
                let embedding_request = toi_server::models::client::EmbeddingRequest {
                    input: description.clone(),
                };
                let embedding = state
                    .model_client
                    .embed(embedding_request)
                    .await
                    .map_err(|(_, err)| err)?;

                // Store all the details.
                let new_openapi_path = toi_server::models::openapi::NewOpenApiPathItem {
                    path: path.to_string(),
                    method: method.to_string(),
                    description,
                    params,
                    body,
                    embedding,
                };
                info!("adding uri={path} method={method}");
                diesel::insert_into(toi_server::schema::openapi::table)
                    .values(&new_openapi_path)
                    .execute(&mut conn)?;
            }
        }
    }

    // Add the main assistant endpoint to the router so it can be included in
    // the docs, but excluded from its own system prompt. Then continue building
    // the API routes.
    let openapi_router =
        openapi_router.nest("/chat", toi_server::routes::chat::router(state.clone()));
    let (router, api) = openapi_router.split_for_parts();
    let router = router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(TraceLayer::new_for_http());

    info!("serving at {}", state.binding_addr);
    let listener = TcpListener::bind(state.binding_addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
