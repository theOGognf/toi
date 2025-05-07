use serde_json::json;
use serial_test::serial;
use std::process::Command;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::search::SimilaritySearchParams;
use toi_server::models::todos::{Todo, TodoQueryParams};

#[tokio::test]
#[serial]
async fn route() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    assert!(db_connection_url.ends_with("/test"));

    // Reset the test database.
    Command::new("diesel")
        .args(["database", "reset"])
        .output()
        .expect("failed to reset test database");

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router =
        OpenApiRouter::new().nest("/todos", toi_server::routes::todos::router(state.clone()));
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await }).await?;
    let client = reqwest::Client::new();

    // Make a todo and get its database-generated ID back.
    let item = "Change my car oil";
    let body = json!(
        {
            "item": item
        }
    );
    let todo1 = client
        .post(format!("{}/todos", state.server_config.bind_addr))
        .json(&body)
        .send()
        .await?
        .json::<Todo>()
        .await?;
    assert_eq!(todo1.item, item);

    // Retrieve the todo using search.
    let query = TodoQueryParams::builder()
        .similarity_search_params(
            SimilaritySearchParams::builder()
                .query("Change my car oil".to_string())
                .distance_threshold(0.5)
                .similarity_threshold(0.5)
                .build(),
        )
        .build();
    let vec_todos1 = client
        .get(format!("{}/todos", state.server_config.bind_addr))
        .query(&query)
        .send()
        .await?
        .json::<Vec<Todo>>()
        .await?;
    assert_eq!(vec_todos1.len(), 1);
    assert_eq!(vec_todos1.first().unwrap().item, item);

    // Delete the todo using search.
    let vec_todos2 = client
        .delete(format!("{}/todos", state.server_config.bind_addr))
        .query(&query)
        .send()
        .await?
        .json::<Vec<Todo>>()
        .await?;
    assert_eq!(vec_todos2.len(), 1);
    assert_eq!(vec_todos2.first().unwrap().item, item);
    Ok(())
}
