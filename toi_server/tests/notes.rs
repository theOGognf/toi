use serde_json::json;
use serial_test::serial;
use std::process::Command;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::notes::{Note, NoteQueryParams};
use toi_server::models::search::SimilaritySearchParams;

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
        OpenApiRouter::new().nest("/notes", toi_server::routes::notes::router(state.clone()));
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.binding_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await }).await?;
    let client = reqwest::Client::new();

    // Make a note and get its database-generated ID back.
    let content = "My car takes OW-20 oil";
    let body = json!(
        {
            "content": content
        }
    );
    let note1 = client
        .post(format!("{}/notes", state.binding_addr))
        .json(&body)
        .send()
        .await?
        .json::<Note>()
        .await?;
    assert_eq!(note1.content, content);

    // Retrieve the note using search.
    let query = NoteQueryParams::builder()
        .similarity_search_params(
            SimilaritySearchParams::builder()
                .query("what's my car's oil type?".to_string())
                .distance_threshold(0.5)
                .similarity_threshold(0.5)
                .build(),
        )
        .build();
    let vec_notes1 = client
        .post(format!("{}/notes", state.binding_addr))
        .query(&query)
        .send()
        .await?
        .json::<Vec<Note>>()
        .await?;
    assert_eq!(vec_notes1.len(), 1);
    assert_eq!(vec_notes1.first().unwrap().content, content);

    // Delete the note using search.
    let vec_notes2 = client
        .delete(format!("{}/notes", state.binding_addr))
        .query(&query)
        .send()
        .await?
        .json::<Vec<Note>>()
        .await?;
    assert_eq!(vec_notes2.len(), 1);
    assert_eq!(vec_notes2.first().unwrap().content, content);
    Ok(())
}
