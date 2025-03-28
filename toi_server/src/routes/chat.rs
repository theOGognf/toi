use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use regex::Regex;
use reqwest::{Client, Request};
use toi::GenerationRequest;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::{
    chat::{
        ChatResponseKind, ExecutedRequests, GeneratedHttpRequest, GeneratedHttpRequests,
        GeneratedPlan, RequestResponse, parse_generated_response,
    },
    client::ModelClientError,
    prompts::{
        DependentHttpRequestPrompt, IndependentHttpRequestsPrompt, PlanPrompt,
        ResponseClassificationPrompt, SimplePrompt, SummaryPrompt, SystemPrompt,
    },
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
    let sysem_prompt = ResponseClassificationPrompt::new(&state.openapi_spec);
    let generation_request = sysem_prompt.into_generation_request(&request.messages);
    let chat_response_kind = state.model_client.generate(generation_request).await?;

    // Parse first integer in string, defaulting to unfulfillable if none is found.
    // In the wild case that an unbounded integer is found, return an error.
    let re = Regex::new(r"/^[^\d]*(\d+)/")
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let chat_response_kind = re
        .find(&chat_response_kind)
        .map(|m| m.as_str())
        .unwrap_or("1");
    let chat_response_kind = chat_response_kind.parse::<u8>().map_err(|err| {
        ModelClientError::ResponseJson.into_response(
            &state.model_client.generation_api_config.base_url,
            &err.to_string(),
        )
    })?;
    let chat_response_kind: ChatResponseKind = chat_response_kind.into();

    // Map the response kind to different prompts and different ways for constructing
    // the final response.
    let generation_request = match chat_response_kind {
        ChatResponseKind::Unfulfillable
        | ChatResponseKind::FollowUp
        | ChatResponseKind::Answer
        | ChatResponseKind::AnswerWithDraftHttpRequests
        | ChatResponseKind::AnswerWithDraftPlan => {
            SimplePrompt::new(&chat_response_kind, &state.openapi_spec)
                .into_generation_request(&request.messages)
        }
        ChatResponseKind::PartiallyAnswerWithHttpRequests
        | ChatResponseKind::AnswerWithHttpRequests => {
            let generation_request =
                IndependentHttpRequestsPrompt::new(&chat_response_kind, &state.openapi_spec)
                    .into_generation_request(&request.messages);
            let generated_http_requests = state.model_client.generate(generation_request).await?;
            let generated_http_requests = parse_generated_response::<GeneratedHttpRequests>(
                generated_http_requests,
                &state.model_client.generation_api_config.base_url,
            )?;
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
            SummaryPrompt::new(&executed_requests).into_generation_request(&request.messages)
        }
        ChatResponseKind::AnswerWithPlan => {
            let generation_request = PlanPrompt::new(&chat_response_kind, &state.openapi_spec)
                .into_generation_request(&request.messages);
            let generated_plan = state.model_client.generate(generation_request).await?;
            let generated_plan = parse_generated_response::<GeneratedPlan>(
                generated_plan,
                &state.model_client.generation_api_config.base_url,
            )?;
            let mut executed_requests = ExecutedRequests::new();
            for generated_http_request_description in generated_plan.plan.iter() {
                let generation_request = DependentHttpRequestPrompt::new(
                    &state.openapi_spec,
                    &generated_plan,
                    &executed_requests,
                    &generated_http_request_description,
                )
                .into_generation_request(&request.messages);
                let generated_http_request =
                    state.model_client.generate(generation_request).await?;
                let generated_http_request = parse_generated_response::<GeneratedHttpRequest>(
                    generated_http_request,
                    &state.model_client.generation_api_config.base_url,
                )?;
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
            SummaryPrompt::new(&executed_requests).into_generation_request(&request.messages)
        }
    };
    let stream = state
        .model_client
        .generate_stream(generation_request)
        .await?;
    Ok(stream)
}
