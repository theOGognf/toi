use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use reqwest::{Client, Request};
use serde_json::{Value, json};
use toi::GenerationRequest;
use tracing::info;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        chat::{GeneratedPlan, GeneratedRequest, OldResponseNewRequest, parse_generated_response},
        client::{EmbeddingRequest, ModelClientError},
        openapi::OpenApiPath,
        prompts::{HttpRequestPrompt, PlanPrompt, SimplePrompt, SummaryPrompt, SystemPrompt},
        state::ToiState,
    },
    schema, utils,
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
    let streaming_generation_request = match request.messages.last() {
        Some(message) => {
            let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
            let embedding_request = EmbeddingRequest {
                input: message.content.clone(),
            };
            info!("embedding request");
            let embedding = state.model_client.embed(embedding_request).await?;
            let result = {
                use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
                use diesel_async::RunQueryDsl;
                use pgvector::VectorExpressionMethods;

                schema::openapi::table
                    .select(OpenApiPath::as_select())
                    .filter(
                        schema::openapi::embedding
                            .cosine_distance(embedding.clone())
                            .le(0.5),
                    )
                    .distinct_on((schema::openapi::path, schema::openapi::method))
                    .load(&mut conn)
                    .await
                    .map_err(utils::internal_error)?
            };
            if result.is_empty() {
                info!("no relevant APIs found");
                SimplePrompt {}.to_streaming_generation_request(&request.messages)
            } else {
                info!("found relevant APIs");
                // Create an OpenAPI spec from the relevant paths retrieved.
                let paths = result.into_iter().map(|p| p.spec).collect::<Vec<Value>>();
                let paths = serde_json::to_value(paths).expect("OpenAPI paths not serializable");
                let spec = json!(
                    {
                        "paths": paths
                    }
                );
                let spec = serde_json::to_string(&spec).expect("OpenAPI spec not serializable");

                // First, plan out requests in response to the user's message.
                info!("making plan from request and relevant APIs");
                let generation_request = PlanPrompt {
                    openapi_spec: &spec,
                }
                .to_generation_request(&request.messages)
                .with_response_format(PlanPrompt::response_format());
                let generated_plan = state.model_client.generate(generation_request).await?;
                let generated_plan = parse_generated_response::<GeneratedPlan>(generated_plan)?;

                // Then, go through and generate each request, using each response
                // as context for the next request.
                info!("executing plan");
                let system_prompt = HttpRequestPrompt {
                    openapi_spec: &spec,
                };
                let mut response_message = None;
                let mut messages = vec![];
                for request in generated_plan.requests.into_iter() {
                    // Adding user message to the pseudo chat for plan execution.
                    let user_message = OldResponseNewRequest {
                        response: response_message,
                        request: Some(request),
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
                        ModelClientError::ApiConnection.into_response(&format!("{err:?}"))
                    })?;
                    response_message = Some(
                        response
                            .text()
                            .await
                            .unwrap_or_else(|err| format!("{err:?}")),
                    );
                }
                // Add the final response for context.
                let user_message = OldResponseNewRequest {
                    response: response_message,
                    request: None,
                }
                .into_user_message();
                messages.push(user_message);

                // The streaming response to the user is a summary of the plan
                // and its execution.
                info!("finalizing plan and summarizing results");
                SummaryPrompt {}.to_streaming_generation_request(&messages)
            }
        }
        None => {
            info!("no messages in request");
            SimplePrompt {}.to_streaming_generation_request(&request.messages)
        }
    };

    info!("begin response stream");
    let stream = state
        .model_client
        .generate_stream(streaming_generation_request)
        .await?;
    Ok(stream)
}
