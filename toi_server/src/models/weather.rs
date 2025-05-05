use bon::Builder;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub enum ForecastLength {
    One,
    Three,
    Seven,
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub enum WeatherCode {
    ClearSky = 0,
    MainlyClear = 1,
    PartlyCloudy = 2,
    Overcast = 3,
    Fog = 45,
    DepositingRimeFog = 48,
    LightDrizzle = 51,
    ModerateDrizzle = 53,
    DenseDrizzle = 55,
    LightFreezingDrizzle = 56,
    DenzeFreezingDrizzle = 57,
    SlightRain = 61,
    ModerateRain = 63,
    HeavyRain = 65,
    LightFreezingRain = 66,
    HeavyFreezingRain = 67,
    SlightSnowFall = 71,
    ModerateSnowFall = 73,
    HeavySnowFall = 75,
    SnowGrains = 77,
    SlightRainShower = 80,
    ModerateRainShower = 81,
    ViolentRainShower = 82,
    SlightSnowShower = 85,
    HeavySnowShower = 86,
    SlightOrModerateThunderstorm = 95,
    ThunderstormWithSlightHail = 96,
    ThunderstormWithHeavyHail = 99,
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct HourlyUnits {
    time: String,
    temperature_2m: String,
    relative_humidity_2m: String,
    precipitation_probability: String,
    precipitation: String,
    weather_code: String,
    wind_speed_10m: String,
    visibility: String,
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct HourlyForecast {
    time: Vec<DateTime<Utc>>,
    temperature_2m: Vec<f64>,
    relative_humidity_2m: Vec<f64>,
    precipitation_probability: Vec<f64>,
    precipitation: Vec<f64>,
    weather_code: Vec<WeatherCode>,
    wind_speed_10m: Vec<f64>,
    visibility: Vec<f64>,
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct WeatherForecast {
    hourly_units: HourlyUnits,
    hourly: HourlyForecast,
}

#[derive(Deserialize)]
pub struct GeocodingResult {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub postcodes: Vec<String>,
    pub country: String,
    pub admin1: String,
    pub admin2: String,
}

#[derive(Deserialize)]
pub struct GeocodingResponse {
    pub results: Vec<GeocodingResult>,
}

impl fmt::Display for GeocodingResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items: Vec<String> = [
            ("Name", &self.name),
            ("Postcodes", &self.postcodes.join(",")),
            ("Administration Detail 1", &self.admin1),
            ("Administration Detail 2", &self.admin2),
            ("Country", &self.country),
        ]
        .iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect();
        let repr = items.join("\n");
        write!(f, "{repr}")
    }
}

#[derive(Builder, Deserialize, Serialize, JsonSchema, IntoParams)]
pub struct WeatherQueryParams {
    /// City to check the weather for.
    pub query: String,
    /// Two-letter country code that the city resides in.
    pub country_code: String,
    /// Number of days of hourly forecast to return.
    pub forecast_length: ForecastLength,
}
