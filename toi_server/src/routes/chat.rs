use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use regex::Regex;
use reqwest::{Client, Request};
use toi::{GenerationRequest, Message, detailed_reqwest_error};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::models::{
    chat::{
        AutoPlan, AutoRequest, AutoRequestSeries, ChatResponseKind, ExecutedRequests,
        RequestResponse, ResponseDescription, parse_generated_response,
    },
    client::ModelClientError,
    prompts::{
        HttpRequestPrompt, IndependentHttpRequestsPrompt, PlanPrompt,
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
    // Search across OpenAPI spec paths for relevant endpoints. If none are
    // found, respond like a normal chat assistant. Otherwise, execute a
    // series of HTTP requests to fulfill the user's request.
    let generation_request = match result {
        None => SimplePrompt {}.to_generation_request(&request.messages),
        Some(paths) => {
            let generation_request = PlanPrompt {
                openapi_spec: &state.openapi_spec,
            }
            .to_generation_request(&request.messages);
            let auto_plan = state.model_client.generate(generation_request).await?;
            let auto_plan = parse_generated_response::<AutoPlan>(auto_plan)?;
            let system_prompt = HttpRequestPrompt {
                openapi_spec: &state.openapi_spec,
            };
            let mut response = None;
            let messages = vec![];
            for auto_request_description in auto_plan.plan.into_iter() {
                let user_message = ResponseDescription {
                    response,
                    description: auto_request_description,
                }
                .to_user_message();
                messages.push(user_message);
                let generation_request = system_prompt.to_generation_request(&messages);
                let auto_request = state.model_client.generate(generation_request).await?;
                let auto_request = parse_generated_response::<AutoRequest>(auto_request)?;
                let request: Request = auto_request.clone().into();
                response = Some(Client::new().execute(request).await.map_err(|err| {
                    ModelClientError::ApiConnection.into_response(&detailed_reqwest_error(err))
                })?);
                let assistant_message = auto_request.to_assistant_message();
                messages.push(assistant_message);
            }
            SummaryPrompt {}.to_generation_request(&messages)
        }
    };

    let stream = state
        .model_client
        .generate_stream(generation_request)
        .await?;
    Ok(stream)
}
