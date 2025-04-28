use diesel::{Connection, PgConnection, RunQueryDsl};
use serde_json::json;
use toi_server::models::prompts::SystemPrompt;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

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

    // Initialize the server state and extract the server binding address.
    let mut conn = PgConnection::establish(&db_connection_url)?;
    let (binding_addr, mut state) = toi_server::init(db_connection_url).await?;

    // Define base router and OpenAPI spec used for building the system prompt
    // for the main assistant endpoint.
    let openapi_router = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/datetime", toi_server::routes::datetime::router())
        .nest("/notes", toi_server::routes::notes::router(state.clone()))
        .nest("/todos", toi_server::routes::todos::router(state.clone()));
    let openapi = openapi_router.get_openapi();

    // Go through and embed all OpenAPI path specs so they can be used as
    // context for generating HTTP requests within the "/chat" endpoint.
    // Start by deleting all the pre-existing OpenAPI path specs just in
    // case.
    diesel::delete(toi_server::schema::openapi::table).execute(&mut conn)?;
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
                let spec = json!(
                    {
                        path.clone(): {
                            method.clone(): op
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

                // Generate some user queries that are related to this API spec.
                // These user queries are then embedding individually and used for
                // API search later. This helps improve odds that a specific user
                // request matches up to the API endpoint.
                let messages = vec![toi::Message {
                    role: toi::MessageRole::User,
                    content: description,
                }];
                let generation_request = toi_server::models::prompts::UserQueryPrompt {}
                    .to_generation_request(&messages)
                    .with_response_format(
                        toi_server::models::prompts::UserQueryPrompt::response_format(),
                    );
                let generated_user_queries = state
                    .model_client
                    .generate(generation_request)
                    .await
                    .map_err(|(_, err)| err)?;
                let generated_user_queries = toi_server::models::chat::parse_generated_response::<
                    toi_server::models::chat::GeneratedUserQueries,
                >(generated_user_queries)
                .map_err(|(_, err)| err)?;
                for input in generated_user_queries.queries {
                    // Embed the endpoint's query.
                    let embedding_request = toi_server::models::client::EmbeddingRequest { input };
                    let embedding = state
                        .model_client
                        .embed(embedding_request)
                        .await
                        .map_err(|(_, err)| err)?;

                    // Store all the details.
                    let new_openapi_path = toi_server::models::openapi::NewOpenApiPath {
                        path: path.to_string(),
                        method: method.clone(),
                        spec: spec.clone(),
                        embedding,
                    };
                    diesel::insert_into(toi_server::schema::openapi::table)
                        .values(&new_openapi_path)
                        .execute(&mut conn)?;
                }
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
