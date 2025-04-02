use std::fmt;
use toi::{GenerationRequest, Message, MessageRole};

use crate::models::chat::{AutoPlan, AutoRequestDescription, ChatResponseKind, ExecutedRequests};

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

pub struct ResponseClassificationPrompt<'a> {
    openapi_spec: &'a str,
}

impl<'a> ResponseClassificationPrompt<'a> {
    pub fn new(openapi_spec: &'a str) -> Self {
        Self { openapi_spec }
    }
}

impl fmt::Display for ResponseClassificationPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let openapi_spec = self.openapi_spec;
        let mut repr = format!(
            r#"
You are personal assistant that helps fulfill a user's request using
an API. The first step is to preprocess the user's message. Given an 
OpenAPI spec and a chat history, your job is to pick from a numbered
list which number is the best response option type.

Here is the OpenAPI spec for reference:

{openapi_spec}

And here are your resposne options:
"#
        );

        for i in 1..=3 {
            let chat_response_kind: ChatResponseKind = i.into();
            repr = format!(
                r#"
{repr}

{i}. {chat_response_kind}"#
            );
        }

        write!(
            f,
            r#"
{repr}

Please only respond with the number that fits best and nothing else!"#
        )
    }
}

pub struct SimplePrompt<'a> {
    chat_response_kind: &'a ChatResponseKind,
    openapi_spec: &'a str,
}

impl<'a> SimplePrompt<'a> {
    pub fn new(chat_response_kind: &'a ChatResponseKind, openapi_spec: &'a str) -> Self {
        Self {
            chat_response_kind,
            openapi_spec,
        }
    }
}

impl fmt::Display for SimplePrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chat_response_kind = self.chat_response_kind.to_string();
        let openapi_spec = self.openapi_spec;
        write!(
            f,
            r#"
You are an assistant that responds given an OpenAPI spec, a chat history, 
and a designated response type.

Here is the OpenAPI spec for reference:

{openapi_spec}

And here is how you should respond:

{chat_response_kind}"#
        )
    }
}

pub struct SummaryPrompt<'a> {
    executed_requests: &'a ExecutedRequests,
}

impl<'a> SummaryPrompt<'a> {
    pub fn new(executed_requests: &'a ExecutedRequests) -> Self {
        Self { executed_requests }
    }
}

impl fmt::Display for SummaryPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let executed_requests = self.executed_requests.to_string();
        write!(
            f,
            r#"
You are an assistant that informs a user what actions were performed by
concisely summarizing HTTP request-responses made in response to a user's
chat history.

Here are the HTTP request-responses:

{executed_requests}"#
        )
    }
}

pub struct IndependentHttpRequestsPrompt<'a> {
    openapi_spec: &'a str,
}

impl<'a> IndependentHttpRequestsPrompt<'a> {
    pub fn new(openapi_spec: &'a str) -> Self {
        Self { openapi_spec }
    }
}

impl fmt::Display for IndependentHttpRequestsPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let openapi_spec = self.openapi_spec;
        write!(
            f,
            r#"
You are an assistant that responds given an OpenAPI spec, a chat history, 
and a designated response type.

Here is the OpenAPI spec for reference:

{openapi_spec}

Only respond with the JSON of the HTTP request(s) and nothing else. The JSON 
should have format:

{{
    "requests": [
        {{
            "method": DELETE/GET/POST/PUT,
            "path": The endpoint path beginning with a forward slash,
            "params": Mapping of query parameter names to their values,
            "body": Mapping of JSON body parameter names to their values,
        }}
    ]
}}"#
        )
    }
}

pub struct PlanPrompt<'a> {
    openapi_spec: &'a str,
}

impl<'a> PlanPrompt<'a> {
    pub fn new(openapi_spec: &'a str) -> Self {
        Self { openapi_spec }
    }
}

impl fmt::Display for PlanPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let openapi_spec = self.openapi_spec;
        write!(
            f,
            r#"
You are an assistant that responds given an OpenAPI spec and a chat history.

Here is the OpenAPI spec for reference:

{openapi_spec}

Respond with a plan of HTTP requests that'll fulfill the user's request.

Only respond with the JSON of the plan and nothing else. The JSON should 
have format:

{{
    "plan": [
        {{
            "method": DELETE/GET/POST/PUT,
            "path": The endpoint path beginning with a forward slash,
            "description": Description of the purpose of this request as part of the plan,
        }}
    ]
}}"#
        )
    }
}

pub struct DependentHttpRequestsPrompt<'a> {
    openapi_spec: &'a str,
    generated_plan: &'a AutoPlan,
    executed_requests: &'a ExecutedRequests,
    generated_http_request_description: &'a AutoRequestDescription,
}

impl<'a> DependentHttpRequestsPrompt<'a> {
    pub fn new(
        openapi_spec: &'a str,
        generated_plan: &'a AutoPlan,
        executed_requests: &'a ExecutedRequests,
        generated_http_request_description: &'a AutoRequestDescription,
    ) -> Self {
        Self {
            openapi_spec,
            generated_plan,
            executed_requests,
            generated_http_request_description,
        }
    }
}

impl fmt::Display for DependentHttpRequestsPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let openapi_spec = self.openapi_spec;
        let generated_plan = self.generated_plan.to_string();
        let executed_requests = self.executed_requests.to_string();
        let generated_http_request_description =
            self.generated_http_request_description.to_string();
        write!(
            f,
            r#"
You are a chat assistant that constructs an HTTP request given an
OpenAPI spec, a chat history, a plan, an executed request history, and a 
description of the HTTP request to make.

Here is the OpenAPI spec:

{openapi_spec}

Here is the plan consisting of HTTP request(s) to make:

{generated_plan}

Here is the history of requests and their respective responses made so far as part of that plan:

{executed_requests}

And here is a description of the next HTTP request to generate:

{generated_http_request_description}

Only respond with the JSON of the HTTP request and nothing else. The JSON 
should have format:

{{
    "method": DELETE/GET/POST/PUT,
    "path": The endpoint path beginning with a forward slash,
    "params": Mapping of query parameter names to their values,
    "body": Mapping of JSON body parameter names to their values,
}}"#
        )
    }
}
