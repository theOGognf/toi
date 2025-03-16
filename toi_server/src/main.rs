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
    response::Json,
    extract::{FromRef, FromRequestParts, State},
    http::{StatusCode, request::Parts},
    routing::get,
};
use diesel::prelude::{QueryDsl, SelectableHelper};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection, RunQueryDsl
};
use tokio::net::TcpListener;

mod models;
use models::Note;
mod schema;
use schema::notes;

type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_connection_str = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:mysecretpassword@localhost:5432/postgres".to_string()
    });

    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_connection_str);
    let pool = bb8::Pool::builder().build(config).await?;

    // build our application with some routes
    let app = Router::new()
        .route(
            "/notes",
            get(list_notes),
        )
        .with_state(pool);

    // run it with hyper
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// we can extract the connection pool with `State`
#[axum::debug_handler]
async fn list_notes(
    State(pool): State<Pool>,
) -> Result<Json<Vec<Note>>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = notes::table.select(Note::as_select()).load(&mut conn).await.map_err(internal_error)?;
    Ok(Json(res))
}

// we can also write a custom extractor that grabs a connection from the pool
// which setup is appropriate depends on your application
struct DatabaseConnection(
    bb8::PooledConnection<'static, AsyncDieselConnectionManager<AsyncPgConnection>>,
);

impl<S> FromRequestParts<S> for DatabaseConnection
where
    S: Send + Sync,
    Pool: FromRef<S>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = Pool::from_ref(state);

        let conn = pool.get_owned().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
