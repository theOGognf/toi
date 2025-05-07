use bon::Builder;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::{IntoParams, ToSchema};

#[derive(Builder, Deserialize, JsonSchema, IntoParams, Serialize)]
pub struct WeatherQueryParams {
    /// Free-form query of where to get weather for. Can be a city, county, zip code, state, or any combination thereof.
    pub query: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointProperties {
    pub forecast: String,
    pub forecast_zone: String,
}

#[derive(Deserialize)]
pub struct Point {
    pub properties: PointProperties,
}

#[derive(Deserialize)]
pub struct GeocodingResult {
    pub name: String,
    pub addresstype: String,
    pub lat: String,
    pub lon: String,
    pub display_name: String,
}

impl fmt::Display for GeocodingResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrecipitationData {
    unit_code: String,
    value: Option<u16>,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GridpointForecastPeriod {
    name: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    temperature: u16,
    temperature_unit: String,
    probability_of_precipitation: PrecipitationData,
    wind_speed: String,
    wind_direction: String,
    short_forecast: String,
    detailed_forecast: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GridpointForecastProperties {
    pub generated_at: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub periods: Vec<GridpointForecastPeriod>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct GridpointForecast {
    properties: GridpointForecastProperties,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ZoneForecastPeriod {
    name: String,
    detailed_forecast: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ZoneForecastProperties {
    pub updated: DateTime<Utc>,
    pub periods: Vec<ZoneForecastPeriod>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ZoneForecast {
    properties: ZoneForecastProperties,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertProperties {
    area_desc: String,
    sent: DateTime<Utc>,
    effective: DateTime<Utc>,
    onset: DateTime<Utc>,
    expires: DateTime<Utc>,
    ends: DateTime<Utc>,
    status: String,
    message_type: String,
    category: String,
    severity: String,
    urgency: String,
    event: String,
    headline: String,
    description: String,
    instruction: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct AlertFeatures {
    properties: AlertProperties,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct WeatherAlerts {
    features: Vec<AlertFeatures>,
}
