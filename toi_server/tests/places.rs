use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::places::{
    NewPlaceRequest, Place, PlaceSearchParams, PlaceUpdates, UpdatePlaceRequest,
};

mod utils;

#[tokio::test]
#[serial]
async fn places_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router = OpenApiRouter::new().nest(
        "/places",
        toi_server::routes::places::places_router(state.clone()),
    );
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let places_url = format!("http://{}/places", state.server_config.bind_addr);

    // Make a place.
    let name = "The Best Pizza Place".to_string();
    let description = "A pizza place downtown".to_string();
    let body = NewPlaceRequest::builder()
        .name(name.clone())
        .description(description.clone())
        .build();
    let response = client.post(&places_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let place1 = response.json::<Place>().await?;
    assert_eq!(place1.name, name);

    // Update the place.
    let phone = "867-5309".to_string();
    let body = UpdatePlaceRequest::builder()
        .place_updates(PlaceUpdates::builder().phone(phone.to_string()).build())
        .build();
    let response = client.put(&places_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let place2 = response.json::<Place>().await?;
    assert_eq!(place2.phone, Some(phone.to_string()));

    // Retrieve the place using search.
    let search_places_url = format!("{places_url}/search");
    let params = PlaceSearchParams::builder()
        .query("pizza places".to_string())
        .build();
    let response = client.post(search_places_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_places1 = response.json::<Vec<Place>>().await?;
    assert_eq!(vec_places1, vec![place2]);

    // Delete the place using search.
    let delete_places_url = format!("{places_url}/delete");
    let response = client.post(delete_places_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_places2 = response.json::<Vec<Place>>().await?;
    assert_eq!(vec_places2, vec_places1);
    Ok(())
}
