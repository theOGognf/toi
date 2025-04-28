use axum::{extract::Query, http::StatusCode, response::Json};
use chrono::{DateTime, Datelike, Duration, Utc};
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::datetime::{DateTimeQueryParams, DateTimeShiftRequest};

pub fn router() -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(now, shift))
        .routes(routes!(weekday));

    let openapi = router.get_openapi_mut();
    let paths = &mut openapi.paths.paths;

    // Update "/shift" extensions
    let datetime_shift_request_json_schema =
        serde_json::to_value(schema_for!(DateTimeShiftRequest)).expect("schema unserializable");
    let shift_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", datetime_shift_request_json_schema)
        .build();
    paths
        .get_mut("/shift")
        .expect("/shift doesn't exist")
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(shift_extensions);

    // Update "/weekday" extensions
    let datetime_query_params_json_schema =
        serde_json::to_value(schema_for!(DateTimeQueryParams)).expect("schema unserializable");
    let weekday_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", datetime_query_params_json_schema)
        .build();
    paths
        .get_mut("/weekday")
        .expect("/weekday doesn't exist")
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(weekday_extensions);

    router
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
    ),
)]
#[axum::debug_handler]
pub async fn weekday(
    Query(params): Query<DateTimeQueryParams>,
) -> Result<String, (StatusCode, String)> {
    let res = params.datetime.weekday();
    Ok(res.to_string())
}
