use chrono::{DateTime, Utc};
use utoipa::{IntoParams, ToSchema};

#[derive(serde::Deserialize, IntoParams)]
pub struct DateTimeParam {
    #[serde(default)]
    pub datetime: DateTime<Utc>,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct DateTimeShiftRequest {
    #[serde(default)]
    pub datetime: DateTime<Utc>,
    #[serde(default)]
    pub weeks: i64,
    #[serde(default)]
    pub days: i64,
    #[serde(default)]
    pub hours: i64,
    #[serde(default)]
    pub minutes: i64,
    #[serde(default)]
    pub seconds: i64,
}
