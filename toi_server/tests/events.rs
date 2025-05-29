use std::str::FromStr;

use chrono::DateTime;
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::events::{Event, EventSearchParams, NewEventRequest};

mod utils;

#[tokio::test]
#[serial]
async fn events_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router = OpenApiRouter::new().nest(
        "/events",
        toi_server::routes::events::events_router(state.clone()),
    );
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let events_url = format!("http://{}/events", state.server_config.bind_addr);

    // Make an event.
    let event_description = "Oil change".to_string();
    let body = NewEventRequest::builder()
        .description(event_description.clone())
        .starts_at(DateTime::from_str("2025-05-08T22:38:38+0000")?)
        .ends_at(DateTime::from_str("2025-05-08T23:38:38+0000")?)
        .build();
    let response = client.post(&events_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let event1 = response.json::<Event>().await?;
    assert_eq!(event1.description, event_description);

    // Retrieve the event using search.
    let search_events_url = format!("{events_url}/search");
    let params = EventSearchParams::builder()
        .query("oil change".to_string())
        .build();
    let response = client.post(search_events_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_events1 = response.json::<Vec<Event>>().await?;
    assert_eq!(vec_events1, vec![event1]);

    // Delete the event using search.
    let delete_events_url = format!("{events_url}/delete");
    let response = client.post(delete_events_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_events2 = response.json::<Vec<Event>>().await?;
    assert_eq!(vec_events2, vec_events1);
    Ok(())
}
