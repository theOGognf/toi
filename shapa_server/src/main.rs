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

use axum::{
    Router,
    extract::{FromRef, FromRequestParts, State},
    http::{StatusCode, request::Parts},
    routing::get,
};
use chrono::Utc;
use pgvector::Vector;
use sqlx::FromRow;
use sqlx::{
    postgres::{PgPool, PgPoolOptions},
    types::chrono::DateTime,
};
use tokio::net::TcpListener;

use std::time::Duration;

#[tokio::main]
async fn main() {
    let db_connection_str = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:mysecretpassword@localhost:5432/postgres".to_string()
    });

    // set up connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    // build our application with some routes
    let app = Router::new()
        .route(
            "/",
            get(using_connection_pool_extractor).post(using_connection_extractor),
        )
        .with_state(pool);

    // run it with hyper
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// we can extract the connection pool with `State`
async fn using_connection_pool_extractor(
    State(pool): State<PgPool>,
) -> Result<String, (StatusCode, String)> {
    sqlx::query_scalar("select 'hello world from pg'")
        .fetch_one(&pool)
        .await
        .map_err(internal_error)
}

// we can also write a custom extractor that grabs a connection from the pool
// which setup is appropriate depends on your application
struct DatabaseConnection(sqlx::pool::PoolConnection<sqlx::Postgres>);

impl<S> FromRequestParts<S> for DatabaseConnection
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);

        let conn = pool.acquire().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "vector")]
struct MyVector(Vector);

impl From<()> for MyVector {
    fn from(value: ()) -> Self {
        MyVector(Vector::from(vec![]))
    }
}

#[derive(Debug)]
struct Note {
    id: i32,
    content: String,
    embedding: Vector,
    created_at: DateTime<Utc>,
}

async fn using_connection_extractor(
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<String, (StatusCode, String)> {
    let foo = sqlx::query_as!(
        Note,
        r#"select id, content, embedding as "embedding: Vector", created_at from notes limit 1"#
    )
    .fetch_one(&mut *conn)
    .await
    .map(|u| u.content)
    .map_err(internal_error);

    foo
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
