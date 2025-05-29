use std::str::FromStr;

use chrono::DateTime;
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::{
    attendees::{AttendeeSearchParams, Attendees},
    contacts::{Contact, NewContactRequest},
    events::{Event, NewEventRequest},
};

mod utils;

#[tokio::test]
#[serial]
async fn attendees_routes() -> Result<(), Box<dyn std::error::Error>> {
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
            toi_server::routes::events::events_router(state.clone()).nest(
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
    let contacts_url = format!("http://{}/contacts", state.server_config.bind_addr);
    let first_name = "Marky mark".to_string();
    let body = NewContactRequest::builder()
        .first_name(first_name.clone())
        .build();
    let response = client.post(&contacts_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let contact1 = response.json::<Contact>().await?;
    assert_eq!(contact1.first_name, first_name);

    // Make an event.
    let events_url = format!("http://{}/events", state.server_config.bind_addr);
    let event_description = "Mark's birthday party".to_string();
    let body = NewEventRequest::builder()
        .description(event_description.clone())
        .starts_at(DateTime::from_str("2025-05-08T22:38:38+0000")?)
        .ends_at(DateTime::from_str("2025-05-08T23:38:38+0000")?)
        .build();
    let response = client.post(&events_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let event1 = response.json::<Event>().await?;
    assert_eq!(event1.description, event_description);

    // Add the contact to the event, making an "attendee".
    let attendees_url = format!("{events_url}/attendees");
    let params = AttendeeSearchParams::builder()
        .event_query("birthday party".to_string())
        .contact_query("marky".to_string())
        .build();
    let response = client.post(&attendees_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let attendees1 = response.json::<Attendees>().await?;
    assert_eq!(attendees1.event, event1);
    assert_eq!(attendees1.contacts, vec![contact1]);

    // Retrieve the attendees using search.
    let search_attendees_url = format!("{events_url}/attendees/search");
    let response = client
        .post(search_attendees_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let attendees2 = response.json::<Attendees>().await?;
    assert_eq!(attendees2, attendees1);

    // Delete the attendees using search.
    let delete_attendees_url = format!("{events_url}/attendees/delete");
    let response = client
        .post(delete_attendees_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let attendees3 = response.json::<Attendees>().await?;
    assert_eq!(attendees3, attendees1);
    Ok(())
}
