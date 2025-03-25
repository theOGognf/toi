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
    Json(request): Json<models::client::GenerationRequest>,
) -> Result<Body, (StatusCode, String)> {
    // First step is classifying the type of response most appropriate based on the
    // user's chat history and last message.
    let sysem_prompt = models::assist::ChatResponseKind::to_kind_system_prompt(state.openapi_spec);
    let generation_request = sysem_prompt.to_generation_request(&request.messages);
    let chat_response_kind = state.client.generate(generation_request).await?;
    let chat_response_kind = chat_response_kind
        .parse::<u8>()
        .map_err(|err| (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()))?;
    let chat_response_kind: models::assist::ChatResponseKind = chat_response_kind.into();

    // Map the response kind to different prompts and different ways for constructing
    // the final response.
    match chat_response_kind {
        models::assist::ChatResponseKind::Unfulfillable
        | models::assist::ChatResponseKind::FollowUp
        | models::assist::ChatResponseKind::Answer
        | models::assist::ChatResponseKind::AnswerWithDraftHttpRequests => {
            let system_prompt = chat_response_kind.to_system_prompt(state.openapi_spec);
            let generation_request = system_prompt.to_generation_request(&request.messages);
            let stream = state.client.generate_stream(generation_request).await?;
            Ok(stream)
        }
        models::assist::ChatResponseKind::PartiallyAnswerWithHttpRequests
        | models::assist::ChatResponseKind::AnswerWithHttpRequests => {
            let system_prompt = chat_response_kind.to_system_prompt(state.openapi_spec);
            let generation_request = system_prompt.to_generation_request(&request.messages);
            let http_requests = state.client.generate(generation_request).await?;
        }
    }
}
