use axum::http::StatusCode;
use reqwest::{Client, Method, Request};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, fmt};
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

#[derive(Deserialize, Serialize)]
pub struct GeneratedRequestInfo {
    method: GeneratedMethod,
    path: String,
    description: String,
}

impl fmt::Display for GeneratedRequestInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = serde_json::to_string_pretty(self).expect("request info is not serializable");
        write!(f, "{repr}")
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GeneratedRequest {
    method: GeneratedMethod,
    path: String,
    params: HashMap<String, String>,
    body: HashMap<String, String>,
}

impl GeneratedRequest {
    pub fn into_assistant_message(self) -> Message {
        Message {
            role: MessageRole::Assistant,
            content: serde_json::to_string_pretty(&self).expect("request is not serializable"),
        }
    }
}

impl From<GeneratedRequest> for Request {
    fn from(val: GeneratedRequest) -> Self {
        Client::new()
            .request(val.method.into(), val.path)
            .query(&val.params)
            .json(&val.body)
            .build()
            .expect("valid request")
    }
}

#[derive(Deserialize, Serialize)]
pub struct GeneratedPlan {
    pub requests: Vec<GeneratedRequestInfo>,
}

impl fmt::Display for GeneratedPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = serde_json::to_string_pretty(self).expect("plan is not serializable");
        write!(f, "{repr}")
    }
}

#[derive(Serialize)]
pub struct OldResponseNewRequest {
    pub response: Option<String>,
    pub request: Option<GeneratedRequestInfo>,
}

impl OldResponseNewRequest {
    pub fn into_user_message(self) -> Message {
        let content = match (&self.response, &self.request) {
            (response, Some(request)) => {
                let description = serde_json::to_string_pretty(&self.request)
                    .expect("request description is not serializable");
                match response {
                    None => description,
                    Some(response) => {
                        format!(
                            r#"
Here's the response from that request:

{response}

And here's the description for the next request:

{request}"#
                        )
                    }
                }
            }
            (Some(response), None) => {
                format!(
                    r#"
Here's the response from that request:

{response}"#
                )
            }
            (None, None) => unreachable!("response or request are always something"),
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
