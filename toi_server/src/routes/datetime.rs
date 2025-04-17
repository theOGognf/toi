use axum::{extract::Query, http::StatusCode, response::Json};
use chrono::{DateTime, Datelike, Duration, Utc};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::datetime::{DateTimeQueryParams, DateTimeShiftRequest};

pub fn router() -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(now, shift))
        .routes(routes!(weekday))
}

/// Get the current time in ISO format.
#[utoipa::path(
    get,
    path = "/now",
    responses(
        (status = 200, description = "Successfully got current date", body = DateTime<Utc>)
    )
)]
#[axum::debug_handler]
pub async fn now() -> Result<Json<DateTime<Utc>>, (StatusCode, String)> {
    let res = chrono::offset::Utc::now();
    Ok(Json(res))
}

/// Shift the given ISO datetime by seconds, minutes, hours, etc.
///
/// Shift the given ISO datetime with the date defaulting to today's date.
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
    Json(datetime_shift_request): Json<DateTimeShiftRequest>,
) -> Result<Json<DateTime<Utc>>, (StatusCode, String)> {
    let res = datetime_shift_request.datetime
        + Duration::weeks(datetime_shift_request.weeks)
        + Duration::days(datetime_shift_request.days)
        + Duration::hours(datetime_shift_request.hours)
        + Duration::minutes(datetime_shift_request.minutes)
        + Duration::seconds(datetime_shift_request.seconds);
    Ok(Json(res))
}

/// Get the weekday of an ISO datetime.
///
/// Get the weekday of an ISO datetime with the date defaulting to today's date.
#[utoipa::path(
    get,
    path = "/weekday",
    params(
        DateTimeQueryParams
    ),
    responses(
        (status = 200, description = "Successfully got weekday of given date", body = String)
    )
)]
#[axum::debug_handler]
pub async fn weekday(
    Query(params): Query<DateTimeQueryParams>,
) -> Result<String, (StatusCode, String)> {
    let res = params.datetime.weekday();
    Ok(res.to_string())
}
