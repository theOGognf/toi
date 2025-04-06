use axum::http::StatusCode;
use reqwest::{Client, Method, Request, Response};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, fmt};
use toi::{Message, MessageRole, detailed_reqwest_error};

use crate::{models::client::ModelClientError, utils};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum AutoMethod {
    Delete,
    Get,
    Post,
    Put,
}

impl From<AutoMethod> for Method {
    fn from(val: AutoMethod) -> Self {
        match val {
            AutoMethod::Delete => Method::DELETE,
            AutoMethod::Get => Method::GET,
            AutoMethod::Post => Method::POST,
            AutoMethod::Put => Method::PUT,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AutoRequestDescription {
    method: AutoMethod,
    path: String,
    description: String,
}

impl fmt::Display for AutoRequestDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = serde_json::to_string_pretty(self).expect("serializable");
        write!(f, "{repr}")
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AutoRequest {
    method: AutoMethod,
    path: String,
    params: HashMap<String, String>,
    body: HashMap<String, String>,
}

impl AutoRequest {
    pub fn to_assistant_message(self) -> Message {
        Message{role: MessageRole::Assistant, content: serde_json::to_string_pretty(&self).expect("serializable")}
    }
}

impl From<AutoRequest> for Request {
    fn from(val: AutoRequest) -> Self {
        Client::new()
            .request(val.method.into(), val.path)
            .query(&val.params)
            .json(&val.body)
            .build()
            .expect("valid request")
    }
}

#[derive(Deserialize)]
pub struct AutoRequestSeries {
    pub requests: Vec<AutoRequest>,
}

#[derive(Deserialize, Serialize)]
pub struct AutoPlan {
    pub plan: Vec<AutoRequestDescription>,
}

impl fmt::Display for AutoPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = serde_json::to_string_pretty(self).expect("serializable");
        write!(f, "{repr}")
    }
}

#[derive(Serialize)]
pub struct ResponseDescriptionPair {
    pub response: Option<Response>,
    pub description: AutoRequestDescription,
}

impl ResponseDescriptionPair {
    pub async fn to_user_message(self) -> Message {
        let description = serde_json::to_string_pretty(&self.description).expect("serializable");
        let content = match self.response {
            None => description,
            Some(response) => {
                let response = response
                    .text()
                    .await
                    .unwrap_or_else(detailed_reqwest_error);
                format!(
                    r#"
Here's the response from that request:

{response}

And here's the description for the next request:

{description}"#
                )
            }
        };
        Message {
            role: MessageRole::User,
            content,
        }
    }
}

pub fn parse_generated_response<T: DeserializeOwned>(s: String) -> Result<T, (StatusCode, String)> {
    let extraction =
        utils::extract_json(&s).map_err(|err| ModelClientError::ResponseJson.into_response(err))?;
    serde_json::from_str::<T>(extraction)
        .map_err(|err| ModelClientError::ResponseJson.into_response(&err.to_string()))
}
