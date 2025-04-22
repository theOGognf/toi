use reqwest::StatusCode;
use serde_json::json;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::notes::{Note, NoteQueryParams, NoteSimilaritySearchParams};

#[tokio::test]
async fn route() -> Result<(), Box<dyn std::error::Error>> {
    let (binding_addr, state) = toi_server::init().await?;
    let openapi_router =
        OpenApiRouter::new().nest("/notes", toi_server::routes::notes::router(state));
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(binding_addr.clone()).await?;

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
        .post(format!("{binding_addr}/notes"))
        .json(&body)
        .send()
        .await?
        .json::<Note>()
        .await?;
    assert_eq!(note1.content, content);

    // Retrieve the note using its ID.
    let note2 = client
        .get(format!("{binding_addr}/notes/{}", note1.id))
        .send()
        .await?
        .json::<Note>()
        .await?;
    assert_eq!(note1.content, note2.content);

    // Delete the note using its ID.
    let status = client
        .delete(format!("{binding_addr}/notes/{}", note1.id))
        .send()
        .await?
        .status();
    assert_eq!(status, StatusCode::OK);

    // Make the note again.
    let note3 = client
        .post(format!("{binding_addr}/notes"))
        .json(&body)
        .send()
        .await?
        .json::<Note>()
        .await?;
    assert_eq!(note3.content, content);

    // Retrieve the note using search.
    let query = NoteQueryParams::builder()
        .similarity_search_params(
            NoteSimilaritySearchParams::builder()
                .query("what's my car's oil type?".to_string())
                .distance_threshold(0.5)
                .build(),
        )
        .build();
    let vec_notes1 = client
        .post(format!("{binding_addr}/notes/search"))
        .query(&query)
        .send()
        .await?
        .json::<Vec<Note>>()
        .await?;
    assert_eq!(vec_notes1.len(), 1);
    assert_eq!(vec_notes1.first().unwrap().content, content);

    // Delete the note using search.
    let vec_notes2 = client
        .delete(format!("{binding_addr}/notes/search"))
        .query(&query)
        .send()
        .await?
        .json::<Vec<Note>>()
        .await?;
    assert_eq!(vec_notes2.len(), 1);
    assert_eq!(vec_notes2.first().unwrap().content, content);
    Ok(())
}
