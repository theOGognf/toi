use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use reqwest::{Client, Request};
use toi::GenerationRequest;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::{
    assist::{
        ChatResponseKind, ExecutedRequests, GeneratedHttpRequest, GeneratedHttpRequests,
        GeneratedPlan, RequestResponse,
    },
    client::ModelClientError,
    state::ToiState,
};

pub fn router(state: ToiState) -> OpenApiRouter {
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
    State(state): State<ToiState>,
    Json(request): Json<GenerationRequest>,
) -> Result<Body, (StatusCode, String)> {
    // First step is classifying the type of response most appropriate based on the
    // user's chat history and last message.
    let sysem_prompt = ChatResponseKind::into_kind_system_prompt(&state.openapi_spec);
    let generation_request = sysem_prompt.into_generation_request(&request.messages);
    let chat_response_kind = state.model_client.generate(generation_request).await?;
    let chat_response_kind = chat_response_kind.parse::<u8>().map_err(|err| {
        ModelClientError::ResponseJson.into_response(
            &state.model_client.generation_api_config.base_url,
            &err.to_string(),
        )
    })?;
    let chat_response_kind: ChatResponseKind = chat_response_kind.into();

    // Map the response kind to different prompts and different ways for constructing
    // the final response.
    let system_prompt = match chat_response_kind {
        ChatResponseKind::Unfulfillable
        | ChatResponseKind::FollowUp
        | ChatResponseKind::Answer
        | ChatResponseKind::AnswerWithDraftHttpRequests
        | ChatResponseKind::AnswerWithDraftPlan => {
            chat_response_kind.into_system_prompt(&state.openapi_spec)
        }
        ChatResponseKind::PartiallyAnswerWithHttpRequests
        | ChatResponseKind::AnswerWithHttpRequests => {
            let system_prompt = chat_response_kind.into_system_prompt(&state.openapi_spec);
            let generation_request = system_prompt.into_generation_request(&request.messages);
            let generated_http_requests = state.model_client.generate(generation_request).await?;
            let generated_http_requests = serde_json::from_str::<GeneratedHttpRequests>(
                &generated_http_requests,
            )
            .map_err(|err| {
                ModelClientError::ResponseJson.into_response(
                    &state.model_client.generation_api_config.base_url,
                    &err.to_string(),
                )
            })?;
            let mut executed_requests = ExecutedRequests::new();
            for generated_http_request in generated_http_requests.requests {
                let request: Request = generated_http_request.clone().into();
                let response = Client::new().execute(request).await.map_err(|err| {
                    ModelClientError::ApiConnection.into_response(
                        &state.model_client.generation_api_config.base_url,
                        &err.to_string(),
                    )
                })?;
                let request_response = RequestResponse {
                    request: generated_http_request,
                    response: response.text().await.unwrap_or_else(|err| err.to_string()),
                };
                executed_requests.push(request_response);
            }
            ChatResponseKind::into_summary_prompt(&executed_requests)
        }
        ChatResponseKind::AnswerWithPlan => {
            let system_prompt = chat_response_kind.into_system_prompt(&state.openapi_spec);
            let generation_request = system_prompt.into_generation_request(&request.messages);
            let generated_plan = state.model_client.generate(generation_request).await?;
            let generated_plan =
                serde_json::from_str::<GeneratedPlan>(&generated_plan).map_err(|err| {
                    ModelClientError::ResponseJson.into_response(
                        &state.model_client.generation_api_config.base_url,
                        &err.to_string(),
                    )
                })?;
            let mut executed_requests = ExecutedRequests::new();
            for generated_http_request_description in generated_plan.plan.iter() {
                let system_prompt = generated_plan.into_system_prompt(
                    &state.openapi_spec,
                    &executed_requests,
                    &generated_http_request_description,
                );
                let generation_request = system_prompt.into_generation_request(&request.messages);
                let generated_http_request =
                    state.model_client.generate(generation_request).await?;
                let generated_http_request = serde_json::from_str::<GeneratedHttpRequest>(
                    &generated_http_request,
                )
                .map_err(|err| {
                    ModelClientError::ResponseJson.into_response(
                        &state.model_client.generation_api_config.base_url,
                        &err.to_string(),
                    )
                })?;
                let request: Request = generated_http_request.clone().into();
                let response = Client::new().execute(request).await.map_err(|err| {
                    ModelClientError::ApiConnection.into_response(
                        &state.model_client.generation_api_config.base_url,
                        &err.to_string(),
                    )
                })?;
                let request_response = RequestResponse {
                    request: generated_http_request,
                    response: response.text().await.unwrap_or_else(|err| err.to_string()),
                };
                executed_requests.push(request_response);
            }
            ChatResponseKind::into_summary_prompt(&executed_requests)
        }
    };
    let generation_request = system_prompt.into_generation_request(&request.messages);
    let stream = state
        .model_client
        .generate_stream(generation_request)
        .await?;
    Ok(stream)
}
