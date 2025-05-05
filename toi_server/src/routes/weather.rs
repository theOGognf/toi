use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use schemars::schema_for;
use serde_json::json;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{ModelClientError, RerankRequest},
        state::ToiState,
        weather::{GeocodingResponse, WeatherForecast, WeatherQueryParams},
    },
    utils,
};

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(get_weather_forecast))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update GET /weather extensions
    let json_schema = schema_for!(WeatherQueryParams);
    let json_schema = serde_json::to_value(json_schema).expect("schema unserializable");
    let extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", json_schema)
        .build();
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(extensions);

    router
}

/// Get the weather forecast.
///
/// Useful for answering phrases that start with the following:
/// - What's the weather like in...
/// - Is it raining today in...
/// - Is it snowing this week in...
/// - How many days is it raining this week in...
#[utoipa::path(
    get,
    path = "",
    params(WeatherQueryParams),
    responses(
        (status = 200, description = "Successfully got weather forecast", body = [WeatherForecast]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_weather_forecast(
    State(state): State<ToiState>,
    Query(params): Query<WeatherQueryParams>,
) -> Result<Json<WeatherForecast>, (StatusCode, String)> {
    let client = reqwest::Client::new();

    // Get latitude/longitude by geocoding the given query.
    let geocoding_params = json!(
        {
            "name": params.query,
            "countryCode": params.country_code,
        }
    );
    let geocoding_response = client
        .post("https://geocoding-api.open-meteo.com/v1/search")
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
        query: params.query.clone(),
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

    // Get weather forecast from the returned latitude/longitude.
    let forecast_params = json!(
        {
            "latitude": latitude,
            "longitude": longitude,
            "hourly": "temperature_2m,relative_humidity_2m,precipitation_probability,precipitation,weather_code,visibility,wind_speed_10m"
        }
    );
    let weather_forecast = client
        .post("https://api.open-meteo.com/v1/forecast")
        .query(&forecast_params)
        .send()
        .await
        .map_err(|err| ModelClientError::ApiConnection.into_response(&format!("{err:?}")))?
        .json::<WeatherForecast>()
        .await
        .map_err(|err| ModelClientError::ResponseJson.into_response(&format!("{err:?}")))?;
    Ok(Json(weather_forecast))
}
