use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use reqwest::Client;
use toi::{GenerationRequest, Message, MessageRole};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        chat::{GeneratedRequest, parse_generated_response},
        client::{EmbeddingPromptTemplate, EmbeddingRequest, ModelClientError},
        openapi::OpenApiPathItem,
        prompts::{HttpRequestPrompt, SimplePrompt, SummaryPrompt, SystemPrompt},
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
    Json(mut request): Json<GenerationRequest>,
) -> Result<Body, (StatusCode, String)> {
    // Search across OpenAPI spec paths for relevant endpoints. If none are
    // found, respond like a normal chat assistant. Otherwise, execute an
    // HTTP request to fulfill the user's request.
    let streaming_generation_request = match request.messages.last() {
        Some(message) => {
            let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
            let input = EmbeddingPromptTemplate::builder()
                .instruction_prefix(
                    "Instruction: Given a user query, retrieve relevant API descriptions for APIs that might be used to answer the user's query"
                        .to_string(),
                )
                .query_prefix("Query: ".to_string())
                .build()
                .apply(message.content.clone());
            let embedding_request = EmbeddingRequest { input };
            let embedding = state.model_client.embed(embedding_request).await?;

            let result = {
                use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
                use diesel_async::RunQueryDsl;
                use pgvector::VectorExpressionMethods;

                let result: Result<OpenApiPathItem, _> = schema::openapi::table
                    .select(OpenApiPathItem::as_select())
                    .filter(schema::openapi::embedding.l2_distance(embedding).le(0.7))
                    .first(&mut conn)
                    .await;

                result
            };
            match result {
                Ok(item) => {
                    // Convert user request into HTTP request.
                    let mut system_prompt: HttpRequestPrompt = item.into();
                    let response_format = system_prompt.response_format();
                    let generation_request = system_prompt
                        .to_generation_request(&request.messages)
                        .with_response_format(response_format);
                    let generated_request = state.model_client.generate(generation_request).await?;
                    let generated_request =
                        parse_generated_response::<GeneratedRequest>(generated_request)?;

                    // Add the HTTP request to the context as an assistant message.
                    let assistant_message = generated_request.clone().into_assistant_message();
                    request.messages.push(assistant_message);

                    // Execute the HTTP request.
                    let http_request = generated_request.into_http_request(state.binding_addr);
                    let response = Client::new().execute(http_request).await.map_err(|err| {
                        ModelClientError::ApiConnection.into_response(&format!("{err:?}"))
                    })?;
                    let content = response
                        .text()
                        .await
                        .unwrap_or_else(|err| format!("{err:?}"));

                    // Add the HTTP response as a pseudo user response.
                    request.messages.push(Message {
                        role: MessageRole::User,
                        content,
                    });
                    SummaryPrompt {}.to_streaming_generation_request(&request.messages)
                }
                Err(_) => SimplePrompt {}.to_streaming_generation_request(&request.messages),
            }
        }
        None => SimplePrompt {}.to_streaming_generation_request(&request.messages),
    };

    let stream = state
        .model_client
        .generate_stream(streaming_generation_request)
        .await?;
    Ok(stream)
}
