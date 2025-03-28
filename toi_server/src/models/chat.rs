use axum::http::StatusCode;
use reqwest::{Client, Method, Request};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, fmt};

use crate::{models::client::ModelClientError, utils};

pub fn parse_generated_response<T: DeserializeOwned>(
    s: String,
    url: &str,
) -> Result<T, (StatusCode, String)> {
    let extraction = utils::extract_json(&s)
        .map_err(|err| ModelClientError::ResponseJson.into_response(url, &err.to_string()))?;
    serde_json::from_str::<T>(&extraction)
        .map_err(|err| ModelClientError::ResponseJson.into_response(url, &err.to_string()))
}

pub enum ChatResponseKind {
    Unfulfillable,
    FollowUp,
    Answer,
    AnswerWithDraftHttpRequests,
    PartiallyAnswerWithHttpRequests,
    AnswerWithHttpRequests,
    AnswerWithDraftPlan,
    AnswerWithPlan,
}

impl fmt::Display for ChatResponseKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Self::Unfulfillable => {
                "Unfulfillable: the user's message cannot accurately be answered \
                or fulfilled. Notify the user."
            }
            Self::FollowUp => {
                "Follow-up: the user's message is unclear and needs clarification \
                before proceeding."
            }
            Self::Answer => {
                "Answer: the user's message is clear and can be answered directly \
                without HTTP request(s). Answer concisely."
            }
            Self::AnswerWithDraftHttpRequests => {
                "Answer with draft HTTP request(s): the user's message requires \
                HTTP request(s), but some details need clarification. Show \
                draft requests for user input."
            }
            Self::PartiallyAnswerWithHttpRequests => {
                "Partially answer with independent HTTP request(s): the user's message is \
                clear and can be accurately fulfilled with independent HTTP request(s), \
                but it's best to only partially fulfill those HTTP request(s) \
                to make sure the user understands exactly what they're \
                requesting. This is good for scenarios where the user is \
                requesting a lot of changes like deleting or adding a lot \
                of resources, and it's best to retrieve the resources first \
                so the user can confirm."
            }
            Self::AnswerWithHttpRequests => {
                "Answer with independent HTTP request(s): the user's message is \
                clear and can be accurately fulfilled with independent HTTP request(s). \
                It's best to make the HTTP request(s) and summarize those \
                requests and their respective responses to the user. This \
                is good for scenarios where the user is requesting small \
                changes like deleting or adding one or two resources."
            }
            Self::AnswerWithDraftPlan => {
                "Answer with a draft of a plan consisting of a bulleted list of \
                descriptions for dependent HTTP request(s) to make on behalf of the user: \
                the user's message indicates they want an action to be performed \
                with a serries of dependent HTTP request(s), but some aspects of \
                the user's message are unclear and could benefit from additional \
                user input. It's best to show a plan draft to the user, summarize \
                it, and then seek the user's input and confirmation."
            }
            Self::AnswerWithPlan => {
                "Answer with a plan consisting of an array of descriptions for dependent HTTP \
                request(s) to make on behalf of the user: the user's message indicates \
                they want an action to be performed with a series of dependent HTTP \
                request(s). This is good for scenarios where the user is requesting \
                small changes like deleting or adding one or two resources, but some \
                HTTP response(s) are needed to construct request bodies or parameters \
                for subsequent HTTP request(s)."
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
            4 => Self::AnswerWithDraftHttpRequests,
            5 => Self::PartiallyAnswerWithHttpRequests,
            6 => Self::AnswerWithHttpRequests,
            7 => Self::AnswerWithDraftPlan,
            8 => Self::AnswerWithPlan,
            _ => Self::Unfulfillable,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum GeneratedHttpMethod {
    Delete,
    Get,
    Post,
    Put,
}

impl From<GeneratedHttpMethod> for Method {
    fn from(val: GeneratedHttpMethod) -> Self {
        match val {
            GeneratedHttpMethod::Delete => Method::DELETE,
            GeneratedHttpMethod::Get => Method::GET,
            GeneratedHttpMethod::Post => Method::POST,
            GeneratedHttpMethod::Put => Method::PUT,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct GeneratedHttpRequestDescription {
    method: GeneratedHttpMethod,
    path: String,
    description: String,
}

impl fmt::Display for GeneratedHttpRequestDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = serde_json::to_string_pretty(self).expect("serializable");
        write!(f, "{repr}")
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GeneratedHttpRequest {
    method: GeneratedHttpMethod,
    path: String,
    params: HashMap<String, String>,
    body: HashMap<String, String>,
}

impl From<GeneratedHttpRequest> for Request {
    fn from(val: GeneratedHttpRequest) -> Self {
        Client::new()
            .request(val.method.into(), val.path)
            .query(&val.params)
            .json(&val.body)
            .build()
            .expect("valid request")
    }
}

#[derive(Deserialize)]
pub struct GeneratedHttpRequests {
    pub requests: Vec<GeneratedHttpRequest>,
}

#[derive(Deserialize, Serialize)]
pub struct GeneratedPlan {
    pub plan: Vec<GeneratedHttpRequestDescription>,
}

impl fmt::Display for GeneratedPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = serde_json::to_string_pretty(self).expect("serializable");
        write!(f, "{repr}")
    }
}

#[derive(Serialize)]
pub struct RequestResponse {
    pub request: GeneratedHttpRequest,
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
