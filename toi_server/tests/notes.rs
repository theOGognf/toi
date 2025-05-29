use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::notes::{NewNoteRequest, Note, NoteSearchParams};

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
    let openapi_router = OpenApiRouter::new().nest(
        "/notes",
        toi_server::routes::notes::notes_router(state.clone()),
    );
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let notes_url = format!("http://{}/notes", state.server_config.bind_addr);

    // Make a note.
    let note_content = "My car takes OW-20 oil".to_string();
    let body = NewNoteRequest::builder()
        .content(note_content.clone())
        .build();
    let response = client.post(&notes_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let note1 = response.json::<Note>().await?;
    assert_eq!(note1.content, note_content);

    // Retrieve the note using search.
    let search_notes_url = format!("{notes_url}/search");
    let params = NoteSearchParams::builder()
        .query("what's my car oil type".to_string())
        .build();
    let response = client.post(search_notes_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_notes1 = response.json::<Vec<Note>>().await?;
    assert_eq!(vec_notes1, vec![note1]);

    // Delete the note using search.
    let delete_notes_url = format!("{notes_url}/delete");
    let response = client.post(delete_notes_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_notes2 = response.json::<Vec<Note>>().await?;
    assert_eq!(vec_notes2, vec_notes1);
    Ok(())
}
