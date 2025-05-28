use chrono::{DateTime, Datelike, Duration, Utc, Weekday};
use serde_json::json;
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::datetime::DateTimeShiftRequest;

mod utils;

#[tokio::test]
#[serial]
async fn datetime_routes() -> Result<(), Box<dyn std::error::Error>> {
    // An explicit database URL is required for setup.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router =
        OpenApiRouter::new().nest("/datetime", toi_server::routes::datetime::datetime_router());
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let url = format!("http://{}/datetime", state.server_config.bind_addr);

    // Get current time and check that the day is correct.
    let now = Utc::now();
    let response = client.get(format!("{url}/now")).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let datetime1 = response.json::<DateTime<Utc>>().await?;
    assert_eq!(datetime1.day(), now.day());

    // Shift the time by a couple of days and then check the day again.
    let body = DateTimeShiftRequest::builder()
        .datetime(now)
        .days(2)
        .build();
    let response = client
        .post(format!("{url}/shift"))
        .json(&body)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let datetime2 = response.json::<DateTime<Utc>>().await?;
    assert_eq!(datetime2.day(), (now + Duration::days(2)).day());

    // Finally, check the weekday of today.
    let params = json!(
        {
            "datetime": now
        }
    );
    let response = client
        .get(format!("{url}/weekday"))
        .query(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let weekday = response.json::<Weekday>().await?;
    assert_eq!(weekday, now.weekday());
    Ok(())
}
