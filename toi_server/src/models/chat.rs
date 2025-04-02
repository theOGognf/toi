use axum::http::StatusCode;
use reqwest::{Client, Method, Request};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, fmt};

use crate::{models::client::ModelClientError, utils};

pub enum ChatResponseKind {
    Unfulfillable,
    FollowUp,
    Answer,
    AnswerWithHttpRequests,
    AnswerWithPlan,
}

impl fmt::Display for ChatResponseKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Self::Unfulfillable => {
                "Unfulfillable: the user's message cannot accurately be answered \
                or fulfilled."
            }
            Self::FollowUp => {
                "Follow-up: the user's message is unclear and needs clarification \
                before proceeding."
            }
            Self::Answer => {
                "Unrelated to API: the user's message is clearly unrelated to the \
                API. Directly respond to the user like a chat assistant."
            }
            Self::AnswerWithHttpRequests => {
                "Related to API: the user's message is clearly related to the API."
            }
            Self::AnswerWithPlan => {
                "Related to API with dependent requests: the user's \
                message is clearly related to the API and requires a series of \
                dependent HTTP request(s)."
            }
        };
        write!(f, "{repr}")
    }
}

impl From<u8> for ChatResponseKind {
    fn from(value: u8) -> Self {
        match value {
            2 => Self::FollowUp,
            3 => Self::Answer,
            4 => Self::AnswerWithHttpRequests,
            5 => Self::AnswerWithPlan,
            _ => Self::Unfulfillable,
        }
    }
}

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
pub struct RequestResponse {
    pub request: AutoRequest,
    pub response: String,
}

#[derive(Serialize)]
pub struct ExecutedRequests {
    pub results: Vec<RequestResponse>,
}

impl ExecutedRequests {
    pub fn new() -> Self {
        Self { results: vec![] }
    }

    pub fn push(&mut self, result: RequestResponse) {
        self.results.push(result);
    }
}

impl fmt::Display for ExecutedRequests {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = serde_json::to_string_pretty(self).expect("serializable");
        write!(f, "{repr}")
    }
}

pub fn parse_generated_response<T: DeserializeOwned>(s: String) -> Result<T, (StatusCode, String)> {
    let extraction =
        utils::extract_json(&s).map_err(|err| ModelClientError::ResponseJson.into_response(err))?;
    serde_json::from_str::<T>(extraction)
        .map_err(|err| ModelClientError::ResponseJson.into_response(&err.to_string()))
}
