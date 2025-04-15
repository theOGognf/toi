use serde_json::{Value, json};
use std::fmt;
use toi::{GenerationRequest, Message, MessageRole};

use crate::models::client::StreamingGenerationRequest;

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
        write!(f, "You are a helpful assistant.")
    }
}

pub struct SummaryPrompt {}

impl fmt::Display for SummaryPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "You are an intelligent assistant that informs a user what actions were performed by concisely summarizing the chat history."
        )
    }
}

pub struct PlanPrompt<'a> {
    pub openapi: &'a str,
}

impl PlanPrompt<'_> {
    pub fn response_format() -> Value {
        json!(
            {
                "type": "json_schema",
                "json_schema": {
                    "name": "plan",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "requests": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "path": {
                                            "type": "string",
                                            "description": "The endpoint path beginning with a forward slash"
                                        },
                                        "method": {
                                            "type": "string",
                                            "description": "The HTTP method to use for the request",
                                            "enum": ["DELETE", "GET", "POST", "PUT"]
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "A description of what this HTTP request is for and how it uses previous HTTP responses (if at all)"
                                        },
                                    },
                                    "additionalProperties": false,
                                    "required": ["path", "method", "description"]
                                }
                            }
                        },
                        "additionalProperties": false,
                        "required": ["requests"]
                    }
                }
            }
        )
    }
}

impl fmt::Display for PlanPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let openapi = self.openapi;
        write!(
            f,
            r#"
You are an intelligent assistant that plans a series of HTTP request(s) given an OpenAPI spec and a chat history.

Here is the OpenAPI spec for reference:

{openapi}"#
        )
    }
}

pub struct HttpRequestPrompt<'a> {
    pub openapi: &'a str,
}

impl HttpRequestPrompt<'_> {
    pub fn response_format() -> Value {
        json!(
            {
                "type": "json_schema",
                "json_schema": {
                    "name": "request",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The endpoint path beginning with a forward slash"
                            },
                            "method": {
                                "type": "string",
                                "description": "The HTTP method to use for the request",
                                "enum": ["DELETE", "GET", "POST", "PUT"]
                            },
                            "params": {
                                "type": "object",
                                "description": "Mapping of query parameter names to their values",
                                "additionalProperties": true
                            },
                            "body": {
                                "type": "object",
                                "description": "Mapping of JSON body parameter names to their values",
                                "additionalProperties": true
                            }
                        },
                        "additionalProperties": false,
                        "required": ["path", "method"]
                    }
                }
            }
        )
    }
}

impl fmt::Display for HttpRequestPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let openapi = self.openapi;
        write!(
            f,
            r#"
You are an intelligent assistant that constructs an HTTP request given an OpenAPI spec, a chat history, and a JSON description of the HTTP request to make.

Here is the OpenAPI spec:

{openapi}"#
        )
    }
}
