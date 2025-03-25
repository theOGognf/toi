use std::{collections::HashMap, fmt};

use serde::{Deserialize, Serialize};
use toi::{Message, MessageRole, GenerationRequest};

pub struct SystemPrompt(String);

impl SystemPrompt {
    pub fn to_generation_request(
        self,
        history: &[toi::Message],
    ) -> GenerationRequest {
        let mut messages = vec![Message {
            role: MessageRole::System,
            content: self.0,
        }];
        messages.extend_from_slice(history);
        GenerationRequest { messages }
    }
}

pub const CHAT_RESPONSE_SYSTEM_PROMPT_INTRO: &'static str = r#"
You are a chat assistant that responds given an OpenAPI spec, a chat history, 
and a designated response type."#;

pub const HTTP_CHAT_RESPONSE_SYSTEM_PROMPT_OUTRO: &'static str = r#"
Only respond with the JSON of the HTTP request(s) and nothing else. The JSON 
should have format:

{
    "requests": [
        {
            "method": DELETE/GET/POST/PUT,
            "path": The endpoint path beginning with a forward slash,
            "params": Mapping of query parameter names to their values,
            "body": Mapping of JSON body parameter names to their values,
        }
    ]
}"#;

pub const KIND_CHAT_RESPONSE_SYSTEM_PROMPT_INTRO: &'static str = r#"
You are a chat assistant that helps preprocess a user's message. Given an 
OpenAPI spec and a chat history, your job is to classify what kind of 
response is best."#;

pub const KIND_CHAT_RESPONSE_SYSTEM_PROMPT_OUTRO: &'static str = r#"
Only respond with the number of the response that fits best and nothing else."#;

pub const SUMMARY_CHAT_RESPONSE_SYSTEM_PROMPT_INTRO: &'static str = r#"
You are a chat assistant that informs a user what actions were performed by
concisely summarizing HTTP request-responses made in response to a user's
request."#;

pub enum ChatResponseKind {
    Unfulfillable,
    FollowUp,
    Answer,
    AnswerWithDraftHttpRequests,
    PartiallyAnswerWithHttpRequests,
    AnswerWithHttpRequests,
}

impl ChatResponseKind {
    pub fn to_kind_system_prompt(openapi_spec: &String) -> SystemPrompt {
        let mut system_prompt = format!(
            r#"
{}

Here is the OpenAPI spec for reference:

{}

And here are your classification options:
"#,
            KIND_CHAT_RESPONSE_SYSTEM_PROMPT_INTRO, openapi_spec
        );

        for i in 1..=6 {
            let chat_response_kind: ChatResponseKind = i.into();
            system_prompt = format!(
                r#"
{system_prompt}
{i}. {chat_response_kind}"#
            );
        }

        system_prompt = format!(
            r#"
{system_prompt}

{KIND_CHAT_RESPONSE_SYSTEM_PROMPT_OUTRO}"#
        );

        SystemPrompt(system_prompt)
    }

    pub fn to_summary_prompt(request_responses: &String) -> SystemPrompt {
        let system_prompt = format!(
            r#"
{SUMMARY_CHAT_RESPONSE_SYSTEM_PROMPT_INTRO}

Here are the HTTP request-responses:

{request_responses}"#
        );

        SystemPrompt(system_prompt)
    }

    pub fn to_system_prompt(self, openapi_spec: &String) -> SystemPrompt {
        let system_prompt = match self {
            Self::Unfulfillable
            | Self::FollowUp
            | Self::Answer
            | Self::AnswerWithDraftHttpRequests => {
                format!(
                    r#"
{}

Here is the OpenAPI spec for reference:

{}

And here is how you should respond:

{}"#,
                    CHAT_RESPONSE_SYSTEM_PROMPT_INTRO,
                    openapi_spec,
                    self.to_string()
                )
            }
            Self::PartiallyAnswerWithHttpRequests | Self::AnswerWithHttpRequests => {
                format!(
                    r#"
{}

Here is the OpenAPI spec for reference:

{}

And here is how you should respond:

{}

{}"#,
                    CHAT_RESPONSE_SYSTEM_PROMPT_INTRO,
                    openapi_spec,
                    self.to_string(),
                    HTTP_CHAT_RESPONSE_SYSTEM_PROMPT_OUTRO,
                )
            }
        };

        SystemPrompt(system_prompt)
    }
}

impl fmt::Display for ChatResponseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Self::Unfulfillable => {
                "Unfulfillable: the user's message cannot accurately \
                be responded to by an answer or fulfilled by HTTP request(s). \
                It's best to notify the user."
            }
            Self::FollowUp => {
                "Follow-up: the user's message cannot be fulfilled by \
                an answer or HTTP request(s). It's best to follow-up to \
                seek clarification."
            }
            Self::Answer => {
                "Answer: the user's message is clear and can be answered \
                directly without HTTP request(s). It's best to concisely \
                answer."
            }
            Self::AnswerWithDraftHttpRequests => {
                "Answer with draft HTTP request(s): the user's message \
                indicates they want an action to be performed with HTTP \
                request(s), but the HTTP request(s) could benefit from \
                user clarifications and/or updates. It's best to show a \
                draft of the HTTP request(s) to the user and seek their \
                input and confirmation."
            }
            Self::PartiallyAnswerWithHttpRequests => {
                "Partially answer with HTTP request(s): the user's message is \
                clear and can be accurately fulfilled with HTTP request(s), \
                but it's best to only partially fulfill those HTTP request(s) \
                to make sure the user understands exactly what they're \
                requesting. This is good for scenarios where the user is \
                requesting a lot of changes like deleting or adding a lot \
                of resources, and it's best to retrieve the resources first \
                so the user can confirm."
            }
            Self::AnswerWithHttpRequests => {
                "Answer with HTTP request(s): the user's message is \
                clear and can be accurately fulfilled with HTTP request(s). \
                It's best to make the HTTP request(s) and summarize those \
                requests and their respective responses to the user. This \
                is good for scenarios where the user is requesting small \
                changes like deleting or adding one or two resources."
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
            _ => Self::Unfulfillable,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum HttpMethod {
    Delete,
    Get,
    Post,
    Put,
}

impl Into<reqwest::Method> for HttpMethod {
    fn into(self) -> reqwest::Method {
        match self {
            Self::Delete => reqwest::Method::DELETE,
            Self::Get => reqwest::Method::GET,
            Self::Post => reqwest::Method::POST,
            Self::Put => reqwest::Method::PUT,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct HttpRequest {
    method: HttpMethod,
    path: String,
    params: HashMap<String, String>,
    body: HashMap<String, String>,
}

impl fmt::Display for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = serde_json::to_string_pretty(self).expect("serializable");
        write!(f, "{repr}")
    }
}

impl Into<reqwest::Request> for HttpRequest {
    fn into(self) -> reqwest::Request {
        reqwest::Client::new()
            .request(self.method.into(), self.path)
            .query(&self.params)
            .json(&self.body)
            .build()
            .expect("valid request")
    }
}

#[derive(Deserialize)]
pub struct HttpRequests {
    pub requests: Vec<HttpRequest>,
}

pub struct RequestResponse {
    pub request: String,
    pub response: String,
}

impl fmt::Display for RequestResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = format!(
            r#"
Request:
{}

Response:
{}"#,
            self.request, self.response
        );
        write!(f, "{repr}")
    }
}
