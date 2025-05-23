use axum::{extract::Query, http::StatusCode, response::Json};
use chrono::{DateTime, Datelike, Duration, Utc, Weekday};
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::datetime::{DateTimeShiftRequest, DateTimeWeekdayParams};

pub fn datetime_router() -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(now, shift))
        .routes(routes!(weekday));

    let openapi = router.get_openapi_mut();
    let paths = &mut openapi.paths.paths;

    // Update GET /weekday extensions
    let json_schema = schema_for!(DateTimeWeekdayParams);
    let json_schema = serde_json::to_value(json_schema).expect("schema unserializable");
    let weekday_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", json_schema)
        .build();
    paths
        .get_mut("/weekday")
        .expect("/weekday doesn't exist")
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(weekday_extensions);

    // Update POST /shift extensions
    let json_schema = schema_for!(DateTimeShiftRequest);
    let json_schema = serde_json::to_value(json_schema).expect("schema unserializable");
    let shift_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", json_schema)
        .build();
    paths
        .get_mut("/shift")
        .expect("/shift doesn't exist")
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(shift_extensions);

    router
}

/// Get the current time.
///
/// Example queries for getting the current time using this endpoint:
/// - What time is it?
/// - Return the time.
/// - Can you get the time?
/// - Do you have the time?
#[utoipa::path(
    get,
    path = "/now",
    responses(
        (status = 200, description = "Successfully got current date", body = DateTime<Utc>)
    )
)]
#[axum::debug_handler]
pub async fn now() -> Result<Json<DateTime<Utc>>, (StatusCode, String)> {
    let res = Utc::now();
    Ok(Json(res))
}

/// Shift the given ISO datetime by seconds, minutes, hours, etc.
///
/// Example queries for shifting time using this endpoint:
/// - What time is it in 30 days?
/// - What day was 10 days ago?
/// - Get the date 22 months and 10 days from now.
/// - Which month was 25 days ago?
#[utoipa::path(
    post,
    path = "/shift", 
    request_body = DateTimeShiftRequest,
    responses(
        (status = 200, description = "Successfully shifted given date", body = DateTime<Utc>)
    )
)]
#[axum::debug_handler]
pub async fn shift(
    Json(body): Json<DateTimeShiftRequest>,
) -> Result<Json<DateTime<Utc>>, (StatusCode, String)> {
    let time_delta = Duration::days(body.days)
        + Duration::weeks(body.weeks)
        + Duration::hours(body.hours)
        + Duration::minutes(body.minutes)
        + Duration::seconds(body.seconds);
    let res = body
        .datetime
        .checked_add_signed(time_delta)
        .ok_or((StatusCode::BAD_REQUEST, "duration overflow".to_string()))?;
    Ok(Json(res))
}

/// Get the weekday of a date.
///
/// Example queries for getting a weekday using this endpoint:
/// - What day of the week is it?
/// - What day of the week is today?
/// - What's the weekday?
/// - Get the weekday.
#[utoipa::path(
    get,
    path = "/weekday",
    params(
        DateTimeWeekdayParams
    ),
    responses(
        (status = 200, description = "Successfully got weekday of given date", body = String)
    ),
)]
#[axum::debug_handler]
pub async fn weekday(
    Query(params): Query<DateTimeWeekdayParams>,
) -> Result<Json<Weekday>, (StatusCode, String)> {
    let res = params.datetime.weekday();
    Ok(Json(res))
}
