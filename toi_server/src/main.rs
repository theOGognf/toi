use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

mod models;
mod routes;
mod schema;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    let api_addr = dotenvy::var("API_ADDR").unwrap_or("127.0.0.1:6969".into());

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_connection_url);
    let pool = bb8::Pool::builder().build(config).await?;

    let (router, api) = OpenApiRouter::new()
        .nest("/datetime", routes::dates::router())
        .nest("/notes", routes::notes::router(pool))
        .split_for_parts();
    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/docs/openapi.json", api));

    let listener = TcpListener::bind(api_addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
