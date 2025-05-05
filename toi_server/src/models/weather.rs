use bon::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use std::fmt;
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize_repr, Serialize, JsonSchema, ToSchema)]
#[repr(u8)]
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
pub struct DailyUnits {
    time: String,
    weather_code: String,
    temperature_2m_min: String,
    temperature_2m_max: String,
    rain_sum: String,
    showers_sum: String,
    snowfall_sum: String,
    precipitation_sum: String,
    precipitation_hours: String,
    relative_humidity_2m_min: String,
    relative_humidity_2m_max: String,
    precipitation_probability_max: String,
    precipitation_probability_min: String,
    visibility_min: String,
    visibility_max: String,
    wind_gusts_10m_min: String,
    wind_gusts_10m_max: String,
    wind_speed_10m_min: String,
    wind_speed_10m_max: String,
    cloud_cover_min: String,
    cloud_cover_max: String,
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct DailyForecast {
    time: Vec<String>,
    weather_code: Vec<WeatherCode>,
    temperature_2m_min: Vec<f32>,
    temperature_2m_max: Vec<f32>,
    rain_sum: Vec<f32>,
    showers_sum: Vec<f32>,
    snowfall_sum: Vec<f32>,
    precipitation_sum: Vec<f32>,
    precipitation_hours: Vec<f32>,
    relative_humidity_2m_min: Vec<u8>,
    relative_humidity_2m_max: Vec<u8>,
    precipitation_probability_max: Vec<u8>,
    precipitation_probability_min: Vec<u8>,
    visibility_min: Vec<f32>,
    visibility_max: Vec<f32>,
    wind_gusts_10m_min: Vec<f32>,
    wind_gusts_10m_max: Vec<f32>,
    wind_speed_10m_min: Vec<f32>,
    wind_speed_10m_max: Vec<f32>,
    cloud_cover_min: Vec<u8>,
    cloud_cover_max: Vec<u8>,
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct DailyWeatherForecast {
    daily_units: DailyUnits,
    daily: DailyForecast,
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
    time: Vec<String>,
    temperature_2m: Vec<f32>,
    relative_humidity_2m: Vec<u8>,
    precipitation_probability: Vec<u8>,
    precipitation: Vec<f32>,
    weather_code: Vec<WeatherCode>,
    wind_speed_10m: Vec<f32>,
    visibility: Vec<f32>,
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct HourlyWeatherForecast {
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
    /// City name to check the weather for (excluding county, state, zip, etc).
    pub city: String,
    /// Two-letter country code that the city resides in.
    pub country_code: String,
}
