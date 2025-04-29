use axum::http::StatusCode;
use reqwest::{Client, Method, Request};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use toi::{Message, MessageRole};

use crate::{models::client::ModelClientError, utils};

#[derive(Deserialize)]
pub struct GeneratedUserQueries {
    pub queries: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum GeneratedMethod {
    Delete,
    Get,
    Post,
    Put,
}

impl From<GeneratedMethod> for Method {
    fn from(val: GeneratedMethod) -> Self {
        match val {
            GeneratedMethod::Delete => Method::DELETE,
            GeneratedMethod::Get => Method::GET,
            GeneratedMethod::Post => Method::POST,
            GeneratedMethod::Put => Method::PUT,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GeneratedRequest {
    method: GeneratedMethod,
    path: String,
    #[serde(default)]
    params: HashMap<String, String>,
    #[serde(default)]
    body: HashMap<String, String>,
}

impl GeneratedRequest {
    pub fn into_assistant_message(self) -> Message {
        Message {
            role: MessageRole::Assistant,
            content: serde_json::to_string_pretty(&self).expect("request is not serializable"),
        }
    }

    pub fn into_http_request(self, binding_addr: String) -> Request {
        Client::new()
            .request(
                self.method.into(),
                format!("http://{binding_addr}{}", self.path),
            )
            .query(&self.params)
            .json(&self.body)
            .build()
            .expect("valid request")
    }
}

pub fn parse_generated_response<T: DeserializeOwned>(s: String) -> Result<T, (StatusCode, String)> {
    let extraction =
        utils::extract_json(&s).map_err(|err| ModelClientError::ResponseJson.into_response(err))?;
    serde_json::from_str::<T>(extraction)
        .map_err(|err| ModelClientError::ResponseJson.into_response(&err.to_string()))
}
