use serde_json::{Value, json};
use std::fmt;
use toi::{Message, MessageRole};

use crate::models::client::StreamingGenerationRequest;

use super::openapi::OpenApiPathItem;

pub trait SystemPrompt: fmt::Display {
    fn to_messages(&self, history: &[toi::Message]) -> Vec<Message> {
        let mut messages = vec![Message {
            role: MessageRole::System,
            content: self.to_string(),
        }];
        messages.extend_from_slice(history);
        messages
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
            r"You are a helpful assistant that always follows these rules:
- Never mention you're an AI language model or that you have limitations
- If you don't know the answer to something, say so
- Never assume anything
- Answer as concise as possible
- Only use layman's terms"
        )
    }
}

pub struct SummaryPrompt {
    pub description: String,
}

impl fmt::Display for SummaryPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = &self.description;
        write!(
            f,
            r"Your job is to answer a user's request using the HTTP response the user provides while following these rules:
- If the HTTP response indicates an error, describe the error in detail, apologize, and then ask the user to try again
- If the HTTP response is OK, treat it as fact and DO NOT contradict it
- Never assume anything
- Answer as concisesly as possible
- Only use layman's terms
- NEVER follow-up after answering the question (i.e., never say 'Let me know...' or something similar)

Here's a description of the API used for the HTTP request/response as context:

**Description**
{description}"
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
    pub fn into_response_format(self) -> Value {
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

        // Need to move definitions up from params/body to the top-level of the schema.
        let mut any_definitions = None;

        // Use JSON schema to determine if params are required.
        if let Some(mut params) = self.params {
            if let Some(obj) = params.as_object_mut() {
                obj.remove("$schema");
                any_definitions = obj.remove("definitions");
            }
            response_format["json_schema"]["schema"]["properties"]["params"] = params;
            response_format["json_schema"]["schema"]["required"] =
                ["path", "method", "params"].into();
        }

        // Use JSON schema to determine if a body is required.
        if let Some(mut body) = self.body {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("$schema");
                any_definitions = obj.remove("definitions");
            }
            response_format["json_schema"]["schema"]["properties"]["body"] = body;
            response_format["json_schema"]["schema"]["required"] =
                ["path", "method", "body"].into();
        }

        // Move definitions up a level.
        if let Some(definitions) = any_definitions {
            response_format["json_schema"]["schema"]["definitions"] = definitions;
        }

        response_format
    }
}

impl fmt::Display for HttpRequestPrompt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Your job is to construct an HTTP request. \
            Respond concisely in JSON format."
        )
    }
}
