use serde_json::json;
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::notes::{Note, NoteQueryParams};
use toi_server::models::search::SimilaritySearchParams;

mod utils;

#[tokio::test]
#[serial]
async fn notes_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router =
        OpenApiRouter::new().nest("/notes", toi_server::routes::notes::router(state.clone()));
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let url = format!("http://{}/notes", state.server_config.bind_addr);

    // Make a note.
    let content = "My car takes OW-20 oil";
    let body = json!(
        {
            "content": content
        }
    );
    let response = client.post(&url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let note1 = response.json::<Note>().await?;
    assert_eq!(note1.content, content);

    // Retrieve the note using search.
    let query = NoteQueryParams::builder()
        .similarity_search_params(
            SimilaritySearchParams::builder()
                .query("what's my car's oil type?".to_string())
                .use_reranking_filter(true)
                .build(),
        )
        .build();
    let response = client.get(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_notes1 = response.json::<Vec<Note>>().await?;
    assert_eq!(vec_notes1.len(), 1);
    assert_eq!(vec_notes1.first(), Some(note1).as_ref());

    // Delete the note using search.
    let response = client.delete(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_notes2 = response.json::<Vec<Note>>().await?;
    assert_eq!(vec_notes2, vec_notes1);
    Ok(())
}
