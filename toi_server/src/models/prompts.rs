use serde_json::{Value, json};
use std::fmt;
use toi::{GenerationRequest, Message, MessageRole};

use crate::models::client::StreamingGenerationRequest;

use super::openapi::OpenApiPathItem;

pub trait SystemPrompt: fmt::Display {
    fn to_generation_request(&self, history: &[toi::Message]) -> GenerationRequest {
        let mut messages = vec![Message {
            role: MessageRole::System,
            content: self.to_string(),
        }];
        messages.extend_from_slice(history);
        GenerationRequest::new(messages)
    }

    fn to_streaming_generation_request(
        &self,
        history: &[toi::Message],
    ) -> StreamingGenerationRequest {
        let mut messages = vec![Message {
            role: MessageRole::System,
            content: self.to_string(),
        }];
        messages.extend_from_slice(history);
        StreamingGenerationRequest::new(messages)
    }
}

impl<T: fmt::Display> SystemPrompt for T {}

pub struct SimplePrompt {}

impl fmt::Display for SimplePrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "You are a helpful assistant, but don't ever mention you're an AI language model or that you have limitations. If you don't know the answer to something, say so."
        )
    }
}

pub struct SummaryPrompt {}

impl fmt::Display for SummaryPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Your job is to concisely summarize the HTTP response the user provides based on the chat history. If the response indicates an error, describe the error in detail, apologize, and then ask the user to try again."
        )
    }
}

pub struct HttpRequestPrompt {
    pub path: String,
    pub method: String,
    pub params: Option<Value>,
    pub body: Option<Value>,
}

impl From<OpenApiPathItem> for HttpRequestPrompt {
    fn from(value: OpenApiPathItem) -> Self {
        Self {
            path: value.path,
            method: value.method,
            params: value.params,
            body: value.body,
        }
    }
}

impl HttpRequestPrompt {
    pub fn response_format(&self) -> Value {
        // The base response format is just the path and method.
        let mut response_format = json!(
            {
                "type": "json_schema",
                "json_schema": {
                    "name": "request",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The endpoint path beginning with a forward slash",
                                "enum": [self.path]
                            },
                            "method": {
                                "type": "string",
                                "description": "The HTTP method to use for the request",
                                "enum": [self.method]
                            }
                        },
                        "additionalProperties": false,
                        "required": ["path", "method"]
                    }
                }
            }
        );

        // Use JSON schema to determine if params are required.
        if let Some(params) = &self.params {
            response_format["json_schema"]["schema"]["properties"]["params"] = params.clone();
            let mut required: Vec<String> = serde_json::from_value(
                response_format["json_schema"]["schema"]["required"].clone(),
            )
            .expect("couldn't deserialize required fields");
            required.push("params".to_string());
            response_format["json_schema"]["schema"]["required"] =
                serde_json::to_value(required).expect("couldn't serialize required fields");
        }

        // Use JSON schema to determine if a body is required.
        if let Some(body) = &self.body {
            response_format["json_schema"]["schema"]["properties"]["body"] = body.clone();
            let mut required: Vec<String> = serde_json::from_value(
                response_format["json_schema"]["schema"]["required"].clone(),
            )
            .expect("couldn't deserialize required fields");
            required.push("body".to_string());
            response_format["json_schema"]["schema"]["required"] =
                serde_json::to_value(required).expect("couldn't serialize required fields");
        }

        response_format
    }
}

impl fmt::Display for HttpRequestPrompt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Your job is to construct an HTTP request. Respond concisely in JSON format."
        )
    }
}
