use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use regex::Regex;
use reqwest::{Client, Request};
use toi::{GenerationRequest, detailed_reqwest_error};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::{
    chat::{
        AutoPlan, AutoRequest, AutoRequestSeries, ChatResponseKind, ExecutedRequests,
        RequestResponse, parse_generated_response,
    },
    client::ModelClientError,
    prompts::{
        DependentHttpRequestsPrompt, IndependentHttpRequestsPrompt, PlanPrompt,
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
    request_body = GenerationRequest,
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
    let generation_request = ResponseClassificationPrompt::new(&state.openapi_spec)
        .to_generation_request(&request.messages);
    let chat_response_kind = state.model_client.generate(generation_request).await?;

    // Parse first integer in string, defaulting to unfulfillable if none is found.
    // In the wild case that an unbounded integer is found, return an error.
    let re = Regex::new(r"/^[^\d]*(\d+)/")
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let chat_response_kind = re
        .find(&chat_response_kind)
        .map(|m| m.as_str())
        .unwrap_or("1");
    let chat_response_kind = chat_response_kind
        .parse::<u8>()
        .map_err(|err| ModelClientError::ResponseJson.into_response(&err.to_string()))?;
    let chat_response_kind: ChatResponseKind = chat_response_kind.into();

    // Map the response kind to different prompts and different ways for constructing
    // the final response.
    let generation_request = match chat_response_kind {
        // We can't procedurally execute anything so just respond with a stream.
        ChatResponseKind::Unfulfillable | ChatResponseKind::FollowUp | ChatResponseKind::Answer => {
            SimplePrompt::new(&chat_response_kind, &state.openapi_spec)
                .to_generation_request(&request.messages)
        }
        // We can procedurally execute a series of HTTP requests, so make those
        // in a series, and then summarize the results with a stream.
        ChatResponseKind::AnswerWithHttpRequests => {
            let generation_request =
                IndependentHttpRequestsPrompt::new(&chat_response_kind, &state.openapi_spec)
                    .to_generation_request(&request.messages);
            let auto_request_series = state.model_client.generate(generation_request).await?;
            let auto_request_series =
                parse_generated_response::<AutoRequestSeries>(auto_request_series)?;
            let mut executed_requests = ExecutedRequests::new();
            for auto_request in auto_request_series.requests {
                let request: Request = auto_request.clone().into();
                let response = Client::new().execute(request).await.map_err(|err| {
                    ModelClientError::ApiConnection.into_response(&detailed_reqwest_error(err))
                })?;
                let request_response = RequestResponse {
                    request: auto_request,
                    response: response.text().await.unwrap_or_else(detailed_reqwest_error),
                };
                executed_requests.push(request_response);
            }
            SummaryPrompt::new(&executed_requests).to_generation_request(&request.messages)
        }
        // We can procedurally execute a series of dependent HTTP requests, so
        // make those in a series, and then summarize the results with a stream.
        ChatResponseKind::AnswerWithPlan => {
            let generation_request = PlanPrompt::new(&chat_response_kind, &state.openapi_spec)
                .to_generation_request(&request.messages);
            let auto_plan = state.model_client.generate(generation_request).await?;
            let auto_plan = parse_generated_response::<AutoPlan>(auto_plan)?;
            let mut executed_requests = ExecutedRequests::new();
            for auto_request_description in auto_plan.plan.iter() {
                let generation_request = DependentHttpRequestsPrompt::new(
                    &state.openapi_spec,
                    &auto_plan,
                    &executed_requests,
                    auto_request_description,
                )
                .to_generation_request(&request.messages);
                let auto_request = state.model_client.generate(generation_request).await?;
                let auto_request = parse_generated_response::<AutoRequest>(auto_request)?;
                let request: Request = auto_request.clone().into();
                let response = Client::new().execute(request).await.map_err(|err| {
                    ModelClientError::ApiConnection.into_response(&detailed_reqwest_error(err))
                })?;
                let request_response = RequestResponse {
                    request: auto_request,
                    response: response.text().await.unwrap_or_else(detailed_reqwest_error),
                };
                executed_requests.push(request_response);
            }
            SummaryPrompt::new(&executed_requests).to_generation_request(&request.messages)
        }
    };
    let stream = state
        .model_client
        .generate_stream(generation_request)
        .await?;
    Ok(stream)
}
