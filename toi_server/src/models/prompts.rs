use std::fmt;
use toi::{GenerationRequest, Message, MessageRole};

pub trait SystemPrompt: fmt::Display {
    fn to_generation_request(&self, history: &[toi::Message]) -> GenerationRequest {
        let mut messages = vec![Message {
            role: MessageRole::System,
            content: self.to_string(),
        }];
        messages.extend_from_slice(history);
        GenerationRequest { messages }
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
            "You are an intelligent assistant that informs a user what \
            actions were performed by concisely summarizing the chat history."
        )
    }
}

pub struct PlanPrompt<'a> {
    pub openapi_spec: &'a str,
}

impl fmt::Display for PlanPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let openapi_spec = self.openapi_spec;
        write!(
            f,
            r#"
You are an intelligent assistant that plans a series of HTTP request(s)
given an OpenAPI spec and a chat history.

Here is the OpenAPI spec for reference:

{openapi_spec}

Only respond with the JSON of the plan and nothing else. The JSON should
have the following format:

{{
    "plan": [
        {{
            "method": DELETE/GET/POST/PUT,
            "path": API path beginning with a forward slash,
            "description": Description of the purpose of this request,
        }}
    ]
}}"#
        )
    }
}

pub struct HttpRequestPrompt<'a> {
    pub openapi_spec: &'a str,
}

impl fmt::Display for HttpRequestPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let openapi_spec = self.openapi_spec;
        write!(
            f,
            r#"
You are an intelligent assistant that constructs an HTTP request given an
OpenAPI spec, a chat history, and a JSON description of the HTTP request
to make.

Here is the OpenAPI spec:

{openapi_spec}

Only respond with the JSON of the HTTP request and nothing else. The JSON 
should have the following format:

{{
    "method": DELETE/GET/POST/PUT,
    "path": The endpoint path beginning with a forward slash,
    "params": Mapping of query parameter names to their values,
    "body": Mapping of JSON body parameter names to their values,
}}"#
        )
    }
}
