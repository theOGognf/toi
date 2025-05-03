use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use reqwest::Client;
use toi::{GenerationRequest, Message, MessageRole};
use tracing::{debug, info, warn};
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

impl From<(&String, &Vec<OpenApiPathItem>)> for RerankRequest {
    fn from(value: (&String, &Vec<OpenApiPathItem>)) -> Self {
        let (query, items) = value;
        let documents = items.iter().map(|item| item.description.clone()).collect();
        Self {
            query: query.to_string(),
            documents,
        }
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
    let streaming_generation_request = if let Some(message) = request.messages.last() {
        debug!(">> {}", message.content);
        let mut conn = state.pool.get().await.map_err(utils::internal_error)?;

        debug!("embedding message for API search");
        let input = EmbeddingPromptTemplate::builder()
            .instruction_prefix(INSTRUCTION_PREFIX.to_string())
            .query_prefix(QUERY_PREFIX.to_string())
            .build()
            .apply(&message.content);
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
        debug!("reranking API search results for relevance");
        let rerank_request: RerankRequest = (&message.content, &items).into();
        let rerank_response = state.model_client.rerank(rerank_request).await?;
        let most_relevant_result = &rerank_response.results[0];
        let item = items.remove(most_relevant_result.index);
        info!(
            "most relevant API (uri={} method={}) scored at {:.3}",
            item.path, item.method, most_relevant_result.relevance_score
        );
        if most_relevant_result.relevance_score >= utils::default_similarity_threshold() {
            debug!("API passes similarity threshold");

            // Convert user request into HTTP request.
            let OpenApiPathItem {
                path,
                method,
                description,
                params,
                body,
            } = item;
            let system_prompt = HttpRequestPrompt {
                path,
                method,
                params,
                body,
            };
            let generation_request = GenerationRequest::builder()
                .messages(system_prompt.to_messages(&request.messages))
                .response_format(system_prompt.into_response_format())
                .build();
            debug!("preparing proxy API request");
            let generated_request = state.model_client.generate(generation_request).await?;
            debug!("parsing proxy API request");
            let generated_request =
                parse_generated_response::<GeneratedRequest>(&generated_request)?;

            // Add the HTTP request to the context as an assistant message.
            let http_request = generated_request.to_http_request(&state.binding_addr);
            let assistant_message = generated_request.into_assistant_message();
            request.messages.push(assistant_message);

            // Execute the HTTP request.
            debug!("sending proxy API request");
            let response = Client::new().execute(http_request).await.map_err(|err| {
                ModelClientError::ApiConnection.into_response(&format!("{err:?}"))
            })?;
            debug!("receiving proxy API response");
            let content = response
                .text()
                .await
                .unwrap_or_else(|err| format!("{err:?}"));

            // Add the HTTP response as a pseudo user response.
            request.messages.push(Message {
                role: MessageRole::User,
                content,
            });
            debug!("summarizing API response");
            SummaryPrompt { description }.to_streaming_generation_request(&request.messages)
        } else {
            debug!("no APIs pass similarity threshold");
            SimplePrompt {}.to_streaming_generation_request(&request.messages)
        }
    } else {
        warn!("no message found in request");
        SimplePrompt {}.to_streaming_generation_request(&request.messages)
    };

    debug!("beginning response stream");
    let stream = state
        .model_client
        .generate_stream(streaming_generation_request)
        .await?;
    Ok(stream)
}
