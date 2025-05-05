use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use reqwest::header;
use schemars::schema_for;
use serde_json::json;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{ModelClientError, RerankRequest},
        state::ToiState,
        weather::{
            DailyWeatherForecast, GeocodingResponse, HourlyWeatherForecast, WeatherQueryParams,
        },
    },
    utils,
};

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(get_daily_weather_forecast))
        .routes(routes!(get_hourly_weather_forecast))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = &mut openapi.paths.paths;

    // Update GET /weather extensions
    let json_schema = schema_for!(WeatherQueryParams);
    let json_schema = serde_json::to_value(json_schema).expect("schema unserializable");
    let extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", json_schema)
        .build();
    paths
        .get_mut("/forecast/daily")
        .expect("doesn't exist")
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(extensions.clone());

    paths
        .get_mut("/forecast/hourly")
        .expect("doesn't exist")
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(extensions);

    router
}

async fn geocode(
    state: &ToiState,
    params: &WeatherQueryParams,
    client: &reqwest::Client,
) -> Result<(f64, f64), (StatusCode, String)> {
    // Get latitude/longitude by geocoding the given query.
    let geocoding_params = json!(
        {
            "name": params.city,
            "countryCode": params.country_code,
        }
    );
    let geocoding_response = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&geocoding_params)
        .send()
        .await
        .map_err(|err| ModelClientError::ApiConnection.into_response(&format!("{err:?}")))?
        .json::<GeocodingResponse>()
        .await
        .map_err(|err| ModelClientError::ResponseJson.into_response(&format!("{err:?}")))?;
    let documents = geocoding_response
        .results
        .iter()
        .map(|r| r.to_string())
        .collect();
    let rerank_request = RerankRequest {
        query: params.city.clone(),
        documents,
    };
    let (latitude, longitude) = state
        .model_client
        .rerank(rerank_request)
        .await?
        .results
        .into_iter()
        .filter(|item| item.relevance_score > utils::default_similarity_threshold())
        .map(|item| {
            let result = &geocoding_response.results[item.index];
            (result.latitude, result.longitude)
        })
        .collect::<Vec<(f64, f64)>>()
        .into_iter()
        .next()
        .ok_or((
            StatusCode::NOT_FOUND,
            "No match found for query".to_string(),
        ))?;

    Ok((latitude, longitude))
}

/// Get the daily weather forecast for one week.
///
/// Useful for answering phrases that start with the following:
/// - What's the weather like this week in...
/// - Is it snowing this week in...
/// - How many days is it raining this week in...
#[utoipa::path(
    get,
    path = "/forecast/daily",
    params(WeatherQueryParams),
    responses(
        (status = 200, description = "Successfully got daily weather forecast", body = [DailyWeatherForecast]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_daily_weather_forecast(
    State(state): State<ToiState>,
    Query(params): Query<WeatherQueryParams>,
) -> Result<Json<DailyWeatherForecast>, (StatusCode, String)> {
    let mut headers = header::HeaderMap::new();
    let user_agent =
        header::HeaderValue::from_str(&state.user_agent).map_err(utils::internal_error)?;
    headers.insert("User-Agent", user_agent);
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(utils::internal_error)?;

    // Get latitude/longitude by geocoding the given query.
    let (latitude, longitude) = geocode(&state, &params, &client).await?;

    // Get weather forecast from the returned latitude/longitude.
    let forecast_params = json!(
        {
            "latitude": latitude,
            "longitude": longitude,
            "daily": "weather_code,temperature_2m_max,rain_sum,showers_sum,snowfall_sum,precipitation_sum,precipitation_hours,precipitation_probability_max,temperature_2m_min,relative_humidity_2m_min,relative_humidity_2m_max,precipitation_probability_min,visibility_min,visibility_max,cloud_cover_max,cloud_cover_min,wind_speed_10m_max,wind_gusts_10m_max,wind_gusts_10m_min,wind_speed_10m_min",
            "forecast_days": 7,
        }
    );
    let weather_forecast = client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&forecast_params)
        .send()
        .await
        .map_err(|err| ModelClientError::ApiConnection.into_response(&format!("{err:?}")))?
        .json::<DailyWeatherForecast>()
        .await
        .map_err(|err| ModelClientError::ResponseJson.into_response(&format!("{err:?}")))?;
    Ok(Json(weather_forecast))
}

/// Get the hourly weather forecast for one day.
///
/// Useful for answering phrases that start with the following:
/// - What's the weather like today in...
/// - Is it raining today in...
/// - Is it snowing today in...
/// - How many hours is it raining today in...
#[utoipa::path(
    get,
    path = "/forecast/hourly",
    params(WeatherQueryParams),
    responses(
        (status = 200, description = "Successfully got hourly weather forecast", body = [HourlyWeatherForecast]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_hourly_weather_forecast(
    State(state): State<ToiState>,
    Query(params): Query<WeatherQueryParams>,
) -> Result<Json<HourlyWeatherForecast>, (StatusCode, String)> {
    let mut headers = header::HeaderMap::new();
    let user_agent =
        header::HeaderValue::from_str(&state.user_agent).map_err(utils::internal_error)?;
    headers.insert("User-Agent", user_agent);
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(utils::internal_error)?;

    // Get latitude/longitude by geocoding the given query.
    let (latitude, longitude) = geocode(&state, &params, &client).await?;

    // Get weather forecast from the returned latitude/longitude.
    let forecast_params = json!(
        {
            "latitude": latitude,
            "longitude": longitude,
            "hourly": "temperature_2m,relative_humidity_2m,precipitation_probability,precipitation,weather_code,visibility,wind_speed_10m",
            "forecast_days": 1
        }
    );
    let weather_forecast = client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&forecast_params)
        .send()
        .await
        .map_err(|err| ModelClientError::ApiConnection.into_response(&format!("{err:?}")))?
        .json::<HourlyWeatherForecast>()
        .await
        .map_err(|err| ModelClientError::ResponseJson.into_response(&format!("{err:?}")))?;
    Ok(Json(weather_forecast))
}
