use serde_json::json;
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::search::SimilaritySearchParams;
use toi_server::models::todos::{Todo, TodoQueryParams};

mod utils;

#[tokio::test]
#[serial]
async fn todos_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router =
        OpenApiRouter::new().nest("/todos", toi_server::routes::todos::router(state.clone()));
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let url = format!("http://{}/todos", state.server_config.bind_addr);

    // Make a todo.
    let item = "Change my car oil";
    let body = json!(
        {
            "item": item
        }
    );
    let response = client.post(&url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let todo1 = response.json::<Todo>().await?;
    assert_eq!(todo1.item, item);

    // Retrieve the todo using search.
    let query = TodoQueryParams::builder()
        .similarity_search_params(
            SimilaritySearchParams::builder()
                .query("Change my car oil".to_string())
                .use_reranking_filter(false)
                .build(),
        )
        .build();
    let response = client.get(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_todos1 = response.json::<Vec<Todo>>().await?;
    assert_eq!(vec_todos1.len(), 1);
    assert_eq!(vec_todos1.first(), Some(todo1).as_ref());

    // Delete the todo using search.
    let response = client.delete(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_todos2 = response.json::<Vec<Todo>>().await?;
    assert_eq!(vec_todos2, vec_todos1);
    Ok(())
}
