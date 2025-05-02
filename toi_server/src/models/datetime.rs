use bon::Builder;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Deserialize, JsonSchema, IntoParams, Serialize)]
#[serde(default)]
pub struct DateTimeWeekdayParams {
    /// Datetime in ISO format. Defaults to now.
    pub datetime: DateTime<Utc>,
}

#[derive(Builder, Default, Deserialize, JsonSchema, Serialize, ToSchema)]
#[serde(default)]
pub struct DateTimeShiftRequest {
    /// Datetime to shift from in ISO format. Defaults to now.
    pub datetime: DateTime<Utc>,
    /// Number of weeks to shift forward (positive) or backward (negative).
    #[builder(default)]
    pub weeks: i64,
    /// Number of days to shift forward (positive) or backward (negative).
    #[builder(default)]
    pub days: i64,
    /// Number of hours to shift forward (positive) or backward (negative).
    #[builder(default)]
    pub hours: i64,
    /// Number of minutes to shift forward (positive) or backward (negative).
    #[builder(default)]
    pub minutes: i64,
    /// Number of seconds to shift forward (positive) or backward (negative).
    #[builder(default)]
    pub seconds: i64,
}
