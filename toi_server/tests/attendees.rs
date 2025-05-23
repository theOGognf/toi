use serde_json::json;
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::{
    attendees::{AttendeeQueryParams, Attendees},
    contacts::Contact,
    events::Event,
};

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
    let openapi_router = OpenApiRouter::new()
        .nest(
            "/contacts",
            toi_server::routes::contacts::contacts_router(state.clone()),
        )
        .nest(
            "/events",
            toi_server::routes::events::event_router(state.clone()).nest(
                "/attendees",
                toi_server::routes::attendees::attendees_router(state.clone()),
            ),
        );
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();

    // Make a contact.
    let url = format!("http://{}/contacts", state.server_config.bind_addr);
    let first_name = "Marky mark";
    let body = json!(
        {
            "first_name": first_name
        }
    );
    let response = client.post(&url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let contact1 = response.json::<Contact>().await?;
    assert_eq!(contact1.first_name, first_name);

    // Make an event.
    let url = format!("http://{}/events", state.server_config.bind_addr);
    let event_description = "Mark's birthday party";
    let body = json!(
        {
            "description": event_description,
            "starts_at": "2025-05-08T22:38:38+0000",
            "ends_at": "2025-05-08T23:38:38+0000"
        }
    );
    let response = client.post(&url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let event1 = response.json::<Event>().await?;
    assert_eq!(event1.description, event_description);

    // Add the contact to the event, making a "attendee".
    let url = format!("http://{}/events/attendees", state.server_config.bind_addr);
    let body = AttendeeQueryParams::builder()
        .event_query("birthday party".to_string())
        .event_use_reranking_filter(false)
        .contact_query("marky".to_string())
        .contact_use_reranking_filter(false)
        .build();
    let response = client.post(&url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let attendees1 = response.json::<Attendees>().await?;
    assert_eq!(attendees1.event, event1);
    assert_eq!(attendees1.contacts.first(), Some(contact1).as_ref());

    // Retrieve the attendees using search.
    let query = AttendeeQueryParams::builder()
        .event_query("birthday party".to_string())
        .event_use_reranking_filter(false)
        .contact_query("marky".to_string())
        .contact_use_reranking_filter(false)
        .build();
    let response = client.get(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let attendees2 = response.json::<Attendees>().await?;
    assert_eq!(attendees2, attendees1);

    // Delete the attendees using search.
    let response = client.delete(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let attendees3 = response.json::<Attendees>().await?;
    assert_eq!(attendees3, attendees1);
    Ok(())
}
