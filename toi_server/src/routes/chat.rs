use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use reqwest::Client;
use toi::{GenerationRequest, Message, MessageRole};
use tracing::{info, warn};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        chat::{GeneratedRequest, parse_generated_response},
        client::{EmbeddingPromptTemplate, EmbeddingRequest, ModelClientError, RerankRequest},
        openapi::OpenApiPathItem,
        prompts::{HttpRequestPrompt, SimplePrompt, SummaryPrompt, SystemPrompt},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str = "Instruction: Given a user query, retrieve RESTful API descriptions based on the command within the user's query";
const QUERY_PREFIX: &str = "Query: ";

impl From<(String, &Vec<OpenApiPathItem>)> for RerankRequest {
    fn from(value: (String, &Vec<OpenApiPathItem>)) -> Self {
        let (query, items) = value;
        let documents = items.iter().map(|item| item.description.clone()).collect();
        Self { query, documents }
    }
}

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
            info!("received request: {}", message.content);
            let mut conn = state.pool.get().await.map_err(utils::internal_error)?;

            info!("embedding request for API search");
            let input = EmbeddingPromptTemplate::builder()
                .instruction_prefix(INSTRUCTION_PREFIX.to_string())
                .query_prefix(QUERY_PREFIX.to_string())
                .build()
                .apply(message.content.clone());
            let embedding_request = EmbeddingRequest { input };
            let embedding = state.model_client.embed(embedding_request).await?;

            let mut items: Vec<OpenApiPathItem> = {
                use diesel::{QueryDsl, SelectableHelper};
                use diesel_async::RunQueryDsl;
                use pgvector::VectorExpressionMethods;

                // There should always be some items returned here.
                schema::openapi::table
                    .select(OpenApiPathItem::as_select())
                    .order(schema::openapi::embedding.cosine_distance(embedding))
                    .limit(5)
                    .load(&mut conn)
                    .await
                    .expect("no APi items found")
            };
            // Rerank the results and reevaluate to see if they're relevant.
            info!("reranking API search results for relevance");
            let rerank_request: RerankRequest = (message.content.clone(), &items).into();
            let rerank_response = state.model_client.rerank(rerank_request).await?;
            let most_relevant_result = &rerank_response.results[0];
            let item = items.remove(most_relevant_result.index);
            info!(
                "most relevant API ({} {}) has score {:.2}",
                item.path, item.method, most_relevant_result.relevance_score
            );
            if most_relevant_result.relevance_score > utils::default_similarity_threshold() {
                info!("API satisfies similarity threshold");

                // Convert user request into HTTP request.
                let mut system_prompt: HttpRequestPrompt = item.into();
                let response_format = system_prompt.response_format();
                let generation_request = system_prompt
                    .to_generation_request(&request.messages)
                    .with_response_format(response_format);
                info!("preparing proxy API request");
                let generated_request = state.model_client.generate(generation_request).await?;
                info!("parsing proxy API request");
                let generated_request =
                    parse_generated_response::<GeneratedRequest>(generated_request)?;

                // Add the HTTP request to the context as an assistant message.
                let assistant_message = generated_request.clone().into_assistant_message();
                request.messages.push(assistant_message);

                // Execute the HTTP request.
                info!("sending proxy API request");
                let http_request = generated_request.into_http_request(state.binding_addr);
                let response = Client::new().execute(http_request).await.map_err(|err| {
                    ModelClientError::ApiConnection.into_response(&format!("{err:?}"))
                })?;
                info!("receiving proxy API response");
                let content = response
                    .text()
                    .await
                    .unwrap_or_else(|err| format!("{err:?}"));

                // Add the HTTP response as a pseudo user response.
                request.messages.push(Message {
                    role: MessageRole::User,
                    content,
                });
                info!("summarizing API response");
                SummaryPrompt {}.to_streaming_generation_request(&request.messages)
            } else {
                info!("no relevant APIs found");
                SimplePrompt {}.to_streaming_generation_request(&request.messages)
            }
        }
        None => {
            warn!("no message found in request");
            SimplePrompt {}.to_streaming_generation_request(&request.messages)
        }
    };

    info!("beginning response stream");
    let stream = state
        .model_client
        .generate_stream(streaming_generation_request)
        .await?;
    Ok(stream)
}
