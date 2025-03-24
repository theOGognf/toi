use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Response},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models;

pub fn router(state: models::state::ToiState) -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(chat)).with_state(state)
}

#[utoipa::path(
    post,
    path = "",
    responses(
        (status = 200, description = "Successfully got a response"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
async fn chat(
    State(state): State<models::state::ToiState>,
    Json(request): Json<models::client::GenerateRequest>,
) -> Result<Body, (StatusCode, String)> {
    // First step is classifying the type of response most appropriate based on the
    // user's chat history and last message.
    let chat_response_kind_system_prompt = chat_response_kind_system_prompt(state.openapi_spec);
    let mut chat_response_kind_messages = vec![models::client::Message {
        role: models::client::MessageRole::System,
        content: chat_response_kind_system_prompt,
    }];
    chat_response_kind_messages.extend_from_slice(&request.messages);
    let chat_response_kind_generation_request = models::client::GenerateRequest {
        messages: chat_response_kind_messages,
    };
    let chat_response_kind = state
        .client
        .generate(chat_response_kind_generation_request)
        .await?;
    let chat_response_kind = chat_response_kind.parse::<u8>()?;
    let chat_response_kind: models::assist::ChatResponseKind = chat_response_kind.into();

    // Map the response kind to different prompts and different ways for constructing
    // the final response.
    match chat_response_kind {
        models::assist::ChatResponseKind::Unfulfillable
        | models::assist::ChatResponseKind::FollowUp
        | models::assist::ChatResponseKind::Answer
        | models::assist::ChatResponseKind::AnswerWithDraftHttpRequests => {
            let chat_response_system_prompt =
                chat_response_system_prompt(state.openapi_spec, chat_response_kind);
            let mut chat_response_messages = vec![models::client::Message {
                role: models::client::MessageRole::System,
                content: chat_response_system_prompt,
            }];
            chat_response_messages.extend_from_slice(&request.messages);
            let chat_response_generation_request = models::client::GenerateRequest {
                messages: chat_response_messages,
            };
            let stream = state
                .client
                .generate_stream(chat_response_generation_request)
                .await?;
            Ok(stream)
        }
        models::assist::ChatResponseKind::PartiallyAnswerWithHttpRequests
        | models::assist::ChatResponseKind::AnswerWithHttpRequests => {
            let chat_response_system_prompt =
                chat_response_http_system_prompt(state.openapi_spec, chat_response_kind);
            let mut chat_response_messages = vec![models::client::Message {
                role: models::client::MessageRole::System,
                content: chat_response_system_prompt,
            }];
            chat_response_messages.extend_from_slice(&request.messages);
            let http_requests_generation_request = models::client::GenerateRequest {
                messages: chat_response_messages,
            };
            let http_requests = state
                .client
                .generate(http_requests_generation_request)
                .await?;
        }
    }
}

fn chat_response_system_prompt(
    openapi_spec: String,
    chat_response_kind: models::assist::ChatResponseKind,
) -> String {
    format!(
        r#"
{}

Here is the OpenAPI spec for reference:

{}

And here is how you should respond:

{}
        "#,
        models::assist::CHAT_RESPONSE_SYSTEM_PROMPT_INTRO,
        openapi_spec,
        &chat_response_kind.to_string()[3..],
    )
}

fn chat_response_http_system_prompt(
    openapi_spec: String,
    chat_response_kind: models::assist::ChatResponseKind,
) -> String {
    format!(
        r#"
{}

Here is the OpenAPI spec for reference:

{}

And here is how you should respond:

{}

{}
        "#,
        models::assist::CHAT_RESPONSE_SYSTEM_PROMPT_INTRO,
        openapi_spec,
        &chat_response_kind.to_string()[3..],
        models::assist::CHAT_RESPONSE_HTTP_SYSTEM_PROMPT_OUTRO,
    )
}

fn chat_response_kind_system_prompt(openapi_spec: String) -> String {
    format!(
        r#"
{}

Here is the OpenAPI spec for reference:

{}

And here are your classification options:

{}
{}
{}
{}
{}

{}
        "#,
        models::assist::CHAT_RESPONSE_KIND_SYSTEM_PROMPT_INTRO,
        openapi_spec,
        models::assist::ChatResponseKind::Unfulfillable,
        models::assist::ChatResponseKind::FollowUp,
        models::assist::ChatResponseKind::Answer,
        models::assist::ChatResponseKind::AnswerWithDraftHttpRequests,
        models::assist::ChatResponseKind::AnswerWithHttpRequests,
        models::assist::CHAT_RESPONSE_KIND_SYSTEM_PROMPT_OUTRO
    )
}
