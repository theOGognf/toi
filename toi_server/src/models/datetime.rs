use chrono::{DateTime, Utc};
use utoipa::{IntoParams, ToSchema};

#[derive(Default, serde::Deserialize, IntoParams)]
#[serde(default)]
pub struct DateTimeQueryParams {
    pub datetime: DateTime<Utc>,
}

#[derive(Default, serde::Deserialize, ToSchema)]
#[serde(default)]
pub struct DateTimeShiftRequest {
    pub datetime: DateTime<Utc>,
    pub weeks: i64,
    pub days: i64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
}
