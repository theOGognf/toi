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
        write!(
            f,
            "You are a helpful assistant, but don't ever mention you're a language model or that you have limitations."
        )
    }
}

pub struct SummaryPrompt {}

impl fmt::Display for SummaryPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Your job is to concisely summarizes the HTTP response the user provides. If the response indicates an error, concisely apologize and ask the user to try again."
        )
    }
}

pub struct UserQueryPrompt {}

impl UserQueryPrompt {
    pub fn response_format() -> Value {
        json!(
            {
                "type": "json_schema",
                "json_schema": {
                    "name": "rewordings",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "queries": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "description": "User query that would result in using the given API based on its description"
                                }
                            }
                        },
                        "additionalProperties": false,
                        "required": ["queries"]
                    }
                }
            }
        )
    }
}

impl fmt::Display for UserQueryPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = concat!(
                "Your job is to take an OpenAPI endpoint description and generate 10 unique user chat questions/commands that would result in using this OpenAPI endpoint based on its description.",
                "\n",
                "\n",
                "Here's an example:",
                "\n",
                "\n",
                "Description: Get the current day in ISO format.",
                "\n",
                "Example generated command: What day is today?",
                "\n",
                "\n",
                "Respond concisely in JSON format."
            ).to_string();
        write!(f, "{repr}")
    }
}

pub struct HttpRequestPrompt<'a> {
    pub openapi_spec: &'a str,
}

impl HttpRequestPrompt<'_> {
    pub fn response_format(path: &String) -> Value {
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
                                "description": "The endpoint path beginning with a forward slash",
                                "enum": [path]
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
        let openapi_spec = self.openapi_spec;
        let repr = format!(
            "{}{}",
            concat!(
                "Your job is to construct an HTTP request given an OpenAPI spec and a chat history. Respond concisely in JSON format.",
                "\n",
                "\n",
                "Here is the OpenAPI spec:",
                "\n"
            ),
            openapi_spec
        );
        write!(f, "{repr}")
    }
}
