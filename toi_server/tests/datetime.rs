use chrono::{DateTime, Datelike, Duration, Utc};
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::datetime::DateTimeShiftRequest;

#[tokio::test]
#[serial]
async fn route() -> Result<(), Box<dyn std::error::Error>> {
    // An explicit database URL is required for setup.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router =
        OpenApiRouter::new().nest("/datetime", toi_server::routes::datetime::router());
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(state.server_config.bind_addr.clone()).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await }).await?;
    let client = reqwest::Client::new();

    // Get current time and check that the day is correct.
    let now = chrono::offset::Utc::now();
    let datetime1 = client
        .get(format!("{}/datetime/now", state.server_config.bind_addr))
        .send()
        .await?
        .json::<DateTime<Utc>>()
        .await?;
    assert_eq!(datetime1.day(), now.day());

    // Shift the time by a couple of days and then check the day again.
    let body = DateTimeShiftRequest::builder()
        .datetime(now)
        .days(2)
        .build();
    let datetime2 = client
        .post(format!("{}/datetime/shift", state.server_config.bind_addr))
        .json(&body)
        .send()
        .await?
        .json::<DateTime<Utc>>()
        .await?;
    assert_eq!(datetime2.day(), (now + Duration::days(2)).day());

    // Finally, check the weekday of today.
    let weekday = client
        .post(format!(
            "{}/datetime/weekday",
            state.server_config.bind_addr
        ))
        .send()
        .await?
        .json::<String>()
        .await?;
    assert_eq!(weekday, now.weekday().to_string());
    Ok(())
}
