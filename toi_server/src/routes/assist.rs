use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use toi::GenerationRequest;
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
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
async fn chat(
    State(state): State<models::state::ToiState>,
    Json(request): Json<GenerationRequest>,
) -> Result<Body, (StatusCode, String)> {
    // First step is classifying the type of response most appropriate based on the
    // user's chat history and last message.
    let sysem_prompt =
        models::assist::ChatResponseKind::into_kind_system_prompt(&state.openapi_spec);
    let generation_request = sysem_prompt.into_generation_request(&request.messages);
    let chat_response_kind = state.client.generate(generation_request).await?;
    let chat_response_kind = chat_response_kind.parse::<u8>().map_err(|err| {
        models::client::ClientError::ResponseJson.into_response(
            &state.client.generation_api_config.base_url,
            &err.to_string(),
        )
    })?;
    let chat_response_kind: models::assist::ChatResponseKind = chat_response_kind.into();

    // Map the response kind to different prompts and different ways for constructing
    // the final response.
    let system_prompt = match chat_response_kind {
        models::assist::ChatResponseKind::Unfulfillable
        | models::assist::ChatResponseKind::FollowUp
        | models::assist::ChatResponseKind::Answer
        | models::assist::ChatResponseKind::AnswerWithDraftHttpRequests => {
            chat_response_kind.into_system_prompt(&state.openapi_spec)
        }
        models::assist::ChatResponseKind::PartiallyAnswerWithHttpRequests
        | models::assist::ChatResponseKind::AnswerWithHttpRequests => {
            let system_prompt = chat_response_kind.into_system_prompt(&state.openapi_spec);
            let generation_request = system_prompt.into_generation_request(&request.messages);
            let http_requests = state.client.generate(generation_request).await?;
            let http_requests = serde_json::from_str::<models::assist::HttpRequests>(
                &http_requests,
            )
            .map_err(|err| {
                models::client::ClientError::ResponseJson.into_response(
                    &state.client.generation_api_config.base_url,
                    &err.to_string(),
                )
            })?;
            let mut request_responses: Vec<String> = vec![];
            for http_request in http_requests.requests {
                let request_repr = http_request.to_string();
                let request: reqwest::Request = http_request.into();
                let response = reqwest::Client::new()
                    .execute(request)
                    .await
                    .map_err(|err| {
                        models::client::ClientError::ApiConnection.into_response(
                            &state.client.generation_api_config.base_url,
                            &err.to_string(),
                        )
                    })?;
                let request_response = models::assist::RequestResponse {
                    request: request_repr,
                    response: response.text().await.unwrap_or_else(|err| err.to_string()),
                };
                request_responses.push(request_response.to_string());
            }
            models::assist::ChatResponseKind::into_summary_prompt(&request_responses.join("\n"))
        }
    };
    let generation_request = system_prompt.into_generation_request(&request.messages);
    let stream = state.client.generate_stream(generation_request).await?;
    Ok(stream)
}
