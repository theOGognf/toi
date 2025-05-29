use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::contacts::{
    Contact, ContactDeleteParams, ContactSearchParams, ContactUpdates, NewContactRequest,
    UpdateContactRequest,
};

mod utils;

#[tokio::test]
#[serial]
async fn contacts_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router = OpenApiRouter::new().nest(
        "/contacts",
        toi_server::routes::contacts::contacts_router(state.clone()),
    );
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let contacts_url = format!("http://{}/contacts", state.server_config.bind_addr);

    // Make a contact.

    let first_name = "Marky mark".to_string();
    let body = NewContactRequest::builder()
        .first_name(first_name.clone())
        .build();
    let response = client.post(&contacts_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let contact1 = response.json::<Contact>().await?;
    assert_eq!(contact1.first_name, first_name);

    // Update the contact.
    let phone = "867-5309".to_string();
    let body = UpdateContactRequest::builder()
        .contact_updates(ContactUpdates::builder().phone(phone.to_string()).build())
        .build();
    let response = client.put(&contacts_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let contact2 = response.json::<Contact>().await?;
    assert_eq!(contact2.phone, Some(phone.to_string()));

    // Retrieve the contact using search.
    let search_contacts_url = format!("{contacts_url}/search");
    let params = ContactSearchParams::builder()
        .query("who is marky mark".to_string())
        .build();
    let response = client
        .post(search_contacts_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_contacts1 = response.json::<Vec<Contact>>().await?;
    assert_eq!(vec_contacts1, vec![contact2]);

    // Delete the contact using search.
    let delete_contacts_url = format!("{contacts_url}/delete");
    let params = ContactDeleteParams::builder()
        .query("who is marky mark".to_string())
        .build();
    let response = client
        .post(delete_contacts_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_contacts2 = response.json::<Vec<Contact>>().await?;
    assert_eq!(vec_contacts2, vec_contacts1);
    Ok(())
}
