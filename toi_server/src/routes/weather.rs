use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use schemars::schema_for;
use serde_json::json;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::{
    client::ApiClientError,
    state::ToiState,
    weather::{
        GeocodingResult, GridpointForecast, Point, WeatherAlerts, WeatherQueryParams, ZoneForecast,
    },
};

pub fn weather_router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(get_weather_alerts))
        .routes(routes!(get_gridpoint_weather_forecast))
        .routes(routes!(get_zone_weather_forecast))
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
        .get_mut("/alerts")
        .expect("doesn't exist")
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(extensions.clone());

    paths
        .get_mut("/forecast/gridpoint")
        .expect("doesn't exist")
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(extensions.clone());

    paths
        .get_mut("/forecast/zone")
        .expect("doesn't exist")
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(extensions);

    router
}

async fn geocode(
    params: &WeatherQueryParams,
    client: &reqwest::Client,
) -> Result<Point, (StatusCode, String)> {
    // Get latitude/longitude by geocoding the given query.
    let geocoding_params = json!(
        {
            "q": params.query,
            "format": "json"
        }
    );
    let mut results = client
        .get("https://nominatim.openstreetmap.org/search")
        .query(&geocoding_params)
        .send()
        .await
        .map_err(|err| ApiClientError::ApiConnection.into_response(&err))?
        .json::<Vec<GeocodingResult>>()
        .await
        .map_err(|err| ApiClientError::ResponseJson.into_response(&err))?;
    if results.is_empty() {
        let err = format!("couldn't geocode {}", params.query);
        return Err(ApiClientError::EmptyResponse.into_response(&err));
    }
    let most_relevant_result = results.swap_remove(0);
    let (latitude, longitude) = (most_relevant_result.lat, most_relevant_result.lon);

    // Get the NWS point from latitude/longitude.
    let point = client
        .get(format!(
            "https://api.weather.gov/points/{latitude},{longitude}"
        ))
        .send()
        .await
        .map_err(|err| ApiClientError::ApiConnection.into_response(&err))?
        .json::<Point>()
        .await
        .map_err(|err| ApiClientError::ResponseJson.into_response(&err))?;

    Ok(point)
}

/// Get weather alerts for an area.
///
/// Example queries for getting weather alerts from this endpoint:
/// - Are there any weather alerts for
/// - Is there a weather alert I should be worried about in
/// - What're the weather warnings for
#[utoipa::path(
    get,
    path = "/alerts",
    params(WeatherQueryParams),
    responses(
        (status = 200, description = "Successfully got weather alerts", body = [WeatherAlerts]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "Forecast zone not found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_weather_alerts(
    State(client): State<reqwest::Client>,
    Query(params): Query<WeatherQueryParams>,
) -> Result<Json<WeatherAlerts>, (StatusCode, String)> {
    // Get metadata about the latitude/longitude point.
    let point = geocode(&params, &client).await?;

    // Get the forecast zone and the weather alerts for that zone
    // from the returned metadata.
    let zone_id = point
        .properties
        .forecast_zone
        .split('/')
        .next_back()
        .ok_or((StatusCode::NOT_FOUND, "forecast zone not found".to_string()))?;
    let url = format!("https://api.weather.gov/alerts/active/zone/{zone_id}");
    let alerts = client
        .get(url)
        .send()
        .await
        .map_err(|err| ApiClientError::ApiConnection.into_response(&err))?
        .json::<WeatherAlerts>()
        .await
        .map_err(|err| ApiClientError::ResponseJson.into_response(&err))?;
    Ok(Json(alerts))
}

/// Get a detailed weather forecast for an area.
///
/// Example queries for getting detailed weather forecast from this endpoint:
/// - What's the detailed weather like
/// - What're the odds of raining today
/// - What's the temperature looking like tomorrow
#[utoipa::path(
    get,
    path = "/forecast/gridpoint",
    params(WeatherQueryParams),
    responses(
        (status = 200, description = "Successfully got gridpoint weather forecast", body = [GridpointForecast]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_gridpoint_weather_forecast(
    State(client): State<reqwest::Client>,
    Query(params): Query<WeatherQueryParams>,
) -> Result<Json<GridpointForecast>, (StatusCode, String)> {
    // Get metadata about the latitude/longitude point.
    let point = geocode(&params, &client).await?;

    // Get weather forecast from the returned metadata.
    let forecast = client
        .get(point.properties.forecast)
        .send()
        .await
        .map_err(|err| ApiClientError::ApiConnection.into_response(&err))?
        .json::<GridpointForecast>()
        .await
        .map_err(|err| ApiClientError::ResponseJson.into_response(&err))?;
    Ok(Json(forecast))
}

/// Get a high-level weather forecast for a broad area.
///
/// Example queries for getting a high-level weather forecast from this endpoint:
/// - What's the weather in the area
/// - Is it sunny in the area
#[utoipa::path(
    get,
    path = "/forecast/zone",
    params(WeatherQueryParams),
    responses(
        (status = 200, description = "Successfully got zone weather forecast", body = [ZoneForecast]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_zone_weather_forecast(
    State(client): State<reqwest::Client>,
    Query(params): Query<WeatherQueryParams>,
) -> Result<Json<ZoneForecast>, (StatusCode, String)> {
    // Get metadata about the latitude/longitude point.
    let point = geocode(&params, &client).await?;

    // Get weather forecast from the returned metadata.
    let forecast = client
        .get(format!("{}/forecast", point.properties.forecast_zone))
        .send()
        .await
        .map_err(|err| ApiClientError::ApiConnection.into_response(&err))?
        .json::<ZoneForecast>()
        .await
        .map_err(|err| ApiClientError::ResponseJson.into_response(&err))?;
    Ok(Json(forecast))
}
