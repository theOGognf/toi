use axum::http::StatusCode;
use reqwest::{Client, Method, Request};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use toi::{Message, MessageRole};

use crate::models::client::ApiClientError;

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GeneratedRequest {
    method: GeneratedMethod,
    path: String,
    params: Option<Value>,
    body: Option<Value>,
}

impl GeneratedRequest {
    #[must_use]
    pub fn into_assistant_message(self) -> Message {
        Message {
            role: MessageRole::Assistant,
            content: serde_json::to_string_pretty(&self).expect("request is not serializable"),
        }
    }

    #[must_use]
    pub fn to_http_request(&self, api_client: &Client, binding_addr: &str) -> Request {
        let mut request_builder = api_client.request(
            self.method.clone().into(),
            format!("http://{binding_addr}{}", self.path),
        );

        if let Some(params) = &self.params {
            request_builder = request_builder.query(params);
        }

        if let Some(body) = &self.body {
            request_builder = request_builder.json(&body);
        }

        request_builder.build().expect("invalid request")
    }
}

pub fn parse_generated_response<T: DeserializeOwned>(s: &str) -> Result<T, (StatusCode, String)> {
    serde_json::from_str::<T>(s).map_err(|err| ApiClientError::ResponseJson.into_response(&err))
}
