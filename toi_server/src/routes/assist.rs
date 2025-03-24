use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Response},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{client, models, schema, state, utils};

pub fn router(openapi_spec: String, state: state::ToiState) -> OpenApiRouter {
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
    State(client): State<client::Client>,
    Json(request): Json<models::client::GenerateRequest>,
) -> Response {
    // Limit chat history size and limit message types to user and assistant messages.
    let chat_history: Vec<models::client::Message> = request
        .messages
        .into_iter()
        .filter(|m| m.role != models::client::MessageRole::System)
        .collect();
    let chat_history_size = chat_history
        .len()
        .saturating_sub(models::client::MAX_CHAT_HISTORY_SIZE);
    let chat_history = &chat_history[chat_history_size..];

    // First step is classifying the type of response most appropriate based on the
    // user's chat history and last message.
    let chat_response_kind_system_prompt = chat_response_kind_system_prompt(openapi_spec);
    let mut chat_response_kind_messages = vec![models::client::Message {
        role: models::client::MessageRole::System,
        content: chat_response_kind_system_prompt,
    }];
    chat_response_kind_messages.extend_from_slice(chat_history);
    let chat_response_kind_request = models::client::GenerateRequest {
        messages: chat_response_kind_messages,
    };
    let chat_response_kind = client.generate(chat_response_kind_request).await?;
    let chat_response_kind = chat_response_kind.parse::<u8>()?;
    let chat_response_kind: models::assist::ChatResponseKind = chat_response_kind.into();

    // Map the response kind to different prompts and different ways for constructing
    // the final response.

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
