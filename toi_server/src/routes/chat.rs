use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use reqwest::{Client, Request};
use toi::{GenerationRequest, detailed_reqwest_error};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::{
    chat::{GeneratedPlan, GeneratedRequest, OldResponseNewRequest, parse_generated_response},
    client::ModelClientError,
    prompts::{HttpRequestPrompt, PlanPrompt, SimplePrompt, SummaryPrompt, SystemPrompt},
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
    // Search across OpenAPI spec paths for relevant endpoints. If none are
    // found, respond like a normal chat assistant. Otherwise, execute a
    // series of HTTP requests to fulfill the user's request.
    let result: Option<String> = None;
    let streaming_generation_request = match result {
        None => SimplePrompt {}.to_streaming_generation_request(&request.messages),
        Some(spec) => {
            // First, plan out requests in response to the user's message.
            let generation_request = PlanPrompt { openapi: &spec }
                .to_generation_request(&request.messages)
                .with_response_format(PlanPrompt::response_format());
            let generated_plan = state.model_client.generate(generation_request).await?;
            let generated_plan = parse_generated_response::<GeneratedPlan>(generated_plan)?;

            // Then, go through and generate each request, using each response
            // as context for the next request.
            let system_prompt = HttpRequestPrompt { openapi: &spec };
            let mut response_message = None;
            let mut messages = vec![];
            for request in generated_plan.requests.into_iter() {
                // Adding user message to the pseudo chat for plan execution.
                let user_message = OldResponseNewRequest {
                    response: response_message,
                    request,
                }
                .into_user_message();
                messages.push(user_message);

                // Generating the actual request and adding it to the pseudo
                // chat.
                let generation_request = system_prompt
                    .to_generation_request(&messages)
                    .with_response_format(HttpRequestPrompt::response_format());
                let generated_request = state.model_client.generate(generation_request).await?;
                let generated_request =
                    parse_generated_response::<GeneratedRequest>(generated_request)?;
                let assistant_message = generated_request.clone().into_assistant_message();
                messages.push(assistant_message);

                // Executing the request and saving the response text for
                // future request context.
                let request: Request = generated_request.into();
                let response = Client::new().execute(request).await.map_err(|err| {
                    ModelClientError::ApiConnection.into_response(&detailed_reqwest_error(err))
                })?;
                response_message =
                    Some(response.text().await.unwrap_or_else(detailed_reqwest_error));
            }
            // The streaming response to the user is a summary of the plan
            // and its execution.
            SummaryPrompt {}.to_streaming_generation_request(&messages)
        }
    };

    let stream = state
        .model_client
        .generate_stream(streaming_generation_request)
        .await?;
    Ok(stream)
}
