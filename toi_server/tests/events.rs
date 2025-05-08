use serde_json::json;
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::events::{Event, EventQueryParams};
use toi_server::models::search::SimilaritySearchParams;

mod utils;

#[tokio::test]
#[serial]
async fn events_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    assert!(db_connection_url.ends_with("/test"));
    utils::reset_database()?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router =
        OpenApiRouter::new().nest("/events", toi_server::routes::events::router(state.clone()));
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let url = format!("http://{}/events", state.server_config.bind_addr);

    // Make an event.
    let description = "Oil change";
    let body = json!(
        {
            "description": description,
            "starts_at": "2025-05-08T22:38:38+0000",
            "ends_at": "2025-05-08T23:38:38+0000"
        }
    );
    let response = client.post(&url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let event1 = response.json::<Event>().await?;
    assert_eq!(event1.description, description);

    // Retrieve the event using search.
    let query = EventQueryParams::builder()
        .similarity_search_params(
            SimilaritySearchParams::builder()
                .query("oil service".to_string())
                .build(),
        )
        .build();
    let response = client.get(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_events1 = response.json::<Vec<Event>>().await?;
    assert_eq!(vec_events1.len(), 1);
    assert_eq!(vec_events1.first().unwrap().description, description);

    // Delete the event using search.
    let response = client.delete(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_events2 = response.json::<Vec<Event>>().await?;
    assert_eq!(vec_events2.len(), 1);
    assert_eq!(vec_events2.first().unwrap().description, description);
    Ok(())
}
