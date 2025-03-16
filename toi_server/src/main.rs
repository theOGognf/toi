//! Example of application using <https://github.com/launchbadge/sqlx>
//!
//! Run with
//!
//! ```not_rust
//! cargo run -p example-sqlx-postgres
//! ```
//!
//! Test with curl:
//!
//! ```not_rust
//! curl 127.0.0.1:3000
//! curl -X POST 127.0.0.1:3000
//! ```

use axum::{Router, routing::get};
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use tokio::net::TcpListener;

mod models;
mod routes;
mod schema;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_connection_str = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:mysecretpassword@localhost:5432/postgres".to_string()
    });

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_connection_str);
    let pool = bb8::Pool::builder().build(config).await?;

    // build our application with some routes
    let app = Router::new()
        .route("/notes", get(routes::notes::list_notes))
        .with_state(pool);

    // run it with hyper
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
