use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct DateTimeQueryParams {
    /// Datetime in ISO format. Defaults to now.
    pub datetime: DateTime<Utc>,
}

#[derive(Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct DateTimeShiftRequest {
    /// Datetime to shift from in ISO format. Defaults to now.
    pub datetime: DateTime<Utc>,
    /// Number of weeks to shift forward (positive) or backward (negative).
    pub weeks: i64,
    /// Number of days to shift forward (positive) or backward (negative).
    pub days: i64,
    /// Number of hours to shift forward (positive) or backward (negative).
    pub hours: i64,
    /// Number of minutes to shift forward (positive) or backward (negative).
    pub minutes: i64,
    /// Number of seconds to shift forward (positive) or backward (negative).
    pub seconds: i64,
}
