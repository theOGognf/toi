use pgvector::Vector;
use sqlx::{Row, postgres::PgPoolOptions};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:mysecretpassword@localhost:5432/postgres")
        .await?;

    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE TABLE IF NOT EXISTS items (id bigserial PRIMARY KEY, embedding vector(3))")
        .execute(&pool)
        .await?;

    let embedding = Vector::from(vec![1.0, 2.0, 3.0]);

    sqlx::query("INSERT INTO items (embedding) VALUES ($1)")
        .bind(&embedding)
        .execute(&pool)
        .await?;

    let rows = sqlx::query("SELECT * FROM items ORDER BY embedding <-> $1 LIMIT 1")
        .bind(&embedding)
        .fetch_all(&pool)
        .await?;

    let embedding: Vector = rows[0].try_get("embedding")?;

    println!("{:?}", embedding);

    // Make a simple query to return the given parameter (use a question mark `?` instead of `$1` for MySQL/MariaDB)
    let row = sqlx::query_scalar!("SELECT $1 as value", "asd")
        .fetch_one(&pool)
        .await?;

    assert_eq!(row.unwrap(), "asd");

    Ok(())
}
