use axum::http::StatusCode;
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::default;
use std::fmt;
use utoipa::ToSchema;

pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;
pub type Conn<'a> = bb8::PooledConnection<
    'a,
    diesel_async::pooled_connection::AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>,
>;

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub enum DateFallsOn {
    Month,
    Week,
    Day,
}

#[derive(Clone, Deserialize, JsonSchema, PartialEq, Serialize, ToSchema)]
pub enum OrderBy {
    Oldest,
    Newest,
}

#[derive(Debug)]
pub enum DeserializeWithEnvSubstError {
    Deserialization(serde_json::Error),
    EnvVarSubstitution(envsubst::Error),
}

impl fmt::Display for DeserializeWithEnvSubstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Self::Deserialization(err) => err.to_string(),
            Self::EnvVarSubstitution(err) => err.to_string(),
        };
        write!(f, "{repr}")
    }
}

fn substitute<'de, D>(value: &mut Value, vars: &HashMap<String, String>) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    match value {
        Value::String(s) => {
            *s = envsubst::substitute(s.clone(), vars)
                .map_err(DeserializeWithEnvSubstError::EnvVarSubstitution)
                .map_err(serde::de::Error::custom)?;
        }
        Value::Array(arr) => {
            for item in arr {
                substitute::<D>(item, vars)?;
            }
        }
        Value::Object(map) => {
            for (_, val) in map {
                substitute::<D>(val, vars)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn deserialize_with_envsubst<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + default::Default,
{
    let mut value: Value = Deserialize::deserialize(deserializer)?;
    if value == Value::Null || (matches!(value, Value::String(ref s) if s.trim().is_empty())) {
        return Ok(T::default());
    }
    let vars: HashMap<String, String> = std::env::vars().collect();
    substitute::<D>(&mut value, &vars)?;
    T::deserialize(value)
        .map_err(DeserializeWithEnvSubstError::Deserialization)
        .map_err(serde::de::Error::custom)
}

pub fn default_distance_threshold() -> f64 {
    0.85
}

pub fn default_similarity_threshold() -> f64 {
    0.45
}

pub fn default_server_binding_addr() -> String {
    "127.0.0.1:6969".to_string()
}

/// Map Diesel errors into a specific response.
pub fn diesel_error(err: diesel::result::Error) -> (StatusCode, String) {
    match err {
        diesel::result::Error::NotFound => (StatusCode::NOT_FOUND, err.to_string()),
        _ => internal_error(err),
    }
}

/// Map any error into a `500 Internal Server Error` response.
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
