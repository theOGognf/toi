use axum::{body::Body, extract::State, http::StatusCode, response::Json};
use reqwest::Client;
use toi::{GenerationRequest, Message, MessageRole};
use tracing::{debug, info, warn};
use utoipa::openapi::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        chat::{GeneratedRequest, parse_generated_response},
        client::{ApiClientError, EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        openapi::{NewSearchableOpenApiPathItem, OpenApiPathItem, SearchableOpenApiPathItem},
        prompts::{HttpRequestPrompt, SimplePrompt, SummaryPrompt, SystemPrompt},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str = "Instruction: Given a user query, retrieve RESTful API descriptions based on the command within the user's query";
const QUERY_PREFIX: &str = "Query: ";

pub async fn router(
    openapi: &mut OpenApi,
    state: ToiState,
) -> Result<OpenApiRouter, Box<dyn std::error::Error>> {
    use diesel_async::RunQueryDsl;
    let mut conn = state.pool.get().await?;

    // Go through and embed all OpenAPI path specs so they can be used as
    // context for generating HTTP requests within the /chat endpoint.
    // Start by deleting all the pre-existing OpenAPI path specs just in
    // case there are any updates.
    info!("preparing OpenAPI endpoints for automation");
    let mut new_searchable_openapi_path_items = vec![];
    diesel::delete(schema::openapi::table)
        .execute(&mut conn)
        .await?;
    for (path, item) in &mut openapi.paths.paths {
        // Parameterized paths are not supported by the /chat endpoint.
        if !path.contains("{") {
            for (method, op) in [
                ("DELETE", &mut item.delete),
                ("GET", &mut item.get),
                ("POST", &mut item.post),
                ("PUT", &mut item.put),
            ] {
                if let Some(op) = op {
                    // Split the docstring according to newlines.
                    let summary_and_description = [op.summary.clone(), op.description.clone()]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<String>>()
                        .join("\n\n");
                    let descriptions: Vec<String> = summary_and_description
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .map(|line| line.to_string())
                        .collect();

                    if descriptions.is_empty() {
                        warn!(
                            "skipping uri={path} method={method} due to missing context from doc string"
                        );
                        continue;
                    }

                    // Get params and body from OpenAPI extensions.
                    let (params, body) = match op.extensions {
                        Some(ref mut extensions) => (
                            extensions.remove("x-json-schema-params"),
                            extensions.remove("x-json-schema-body"),
                        ),
                        None => (None, None),
                    };

                    // Make sure the JSON schema params match the params in the OpenAPI spec.
                    match (&op.parameters, &params) {
                        (Some(_), Some(_)) | (None, None) => {}
                        (Some(_), None) => {
                            warn!(
                                "skipping uri={path} method={method} due to missing JSON schema parameters"
                            );
                            continue;
                        }
                        (None, Some(_)) => {
                            warn!(
                                "skipping uri={path} method={method} due to extra JSON schema parameters"
                            );
                            continue;
                        }
                    }

                    // Make sure the JSON schema bodies match the bodies in the OpenAPI spec.
                    match (&op.request_body, &body) {
                        (Some(_), Some(_)) | (None, None) => {}
                        (Some(_), None) => {
                            warn!(
                                "skipping uri={path} method={method} due to missing JSON schema body"
                            );
                            continue;
                        }
                        (None, Some(_)) => {
                            warn!(
                                "skipping uri={path} method={method} due to extra JSON schema body"
                            );
                            continue;
                        }
                    }

                    // Assuming that each line in the endpoint's docstring has
                    // a more unique string that might better match up to a user's
                    // query, each line in an endpoint's docstring is used as a
                    // separate embedding.
                    info!("adding uri={path} method={method}");
                    let new_openapi_path_item = OpenApiPathItem {
                        path: path.to_string(),
                        method: method.to_string(),
                        description: summary_and_description,
                        params,
                        body,
                    };
                    let parent_id = diesel::insert_into(schema::openapi::table)
                        .values(&new_openapi_path_item)
                        .returning(schema::openapi::id)
                        .get_result(&mut conn)
                        .await?;
                    for description in descriptions {
                        debug!("processing line='{description}'");
                        let embedding_request = EmbeddingRequest {
                            input: description.clone(),
                        };
                        let embedding = state
                            .model_client
                            .embed(embedding_request)
                            .await
                            .map_err(|(_, err)| err)?;
                        new_searchable_openapi_path_items.push(NewSearchableOpenApiPathItem {
                            parent_id,
                            description,
                            embedding,
                        });
                    }
                }
            }
        }
    }
    diesel::insert_into(schema::searchable_openapi::table)
        .values(&new_searchable_openapi_path_items)
        .execute(&mut conn)
        .await?;
    drop(conn);

    Ok(OpenApiRouter::new().routes(routes!(chat)).with_state(state))
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

        let items: Vec<SearchableOpenApiPathItem> = {
            use diesel::{QueryDsl, SelectableHelper};
            use diesel_async::RunQueryDsl;
            use pgvector::VectorExpressionMethods;

            // There should always be some items returned here.
            schema::searchable_openapi::table
                .select(SearchableOpenApiPathItem::as_select())
                .order(schema::searchable_openapi::embedding.cosine_distance(embedding))
                .limit(16)
                .load(&mut conn)
                .await
                .expect("no APi items found")
        };
        // Rerank the results and reevaluate to see if they're relevant.
        debug!("reranking API search results for relevance");
        let (mut ids, documents): (Vec<i32>, Vec<String>) = items
            .into_iter()
            .map(|item| (item.parent_id, item.description))
            .unzip();
        let rerank_request = RerankRequest {
            query: message.content.clone(),
            documents,
        };
        let rerank_response = state.model_client.rerank(rerank_request).await?;
        let most_relevant_result = &rerank_response.results[0];
        let parent_id = ids.swap_remove(most_relevant_result.index);
        let item: OpenApiPathItem = {
            use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
            use diesel_async::RunQueryDsl;

            // There should always be some items returned here.
            schema::openapi::table
                .select(OpenApiPathItem::as_select())
                .filter(schema::openapi::id.eq(parent_id))
                .get_result(&mut conn)
                .await
                .expect("APi item found")
        };

        info!(
            "most relevant API (uri={} method={}) scored at {:.3}",
            item.path, item.method, most_relevant_result.relevance_score
        );
        if most_relevant_result.relevance_score >= state.server_config.similarity_threshold {
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
            debug!("proxy API request={:?}", generated_request);

            // Add the HTTP request to the context as an assistant message.
            let http_request = generated_request.to_http_request(&state.server_config.bind_addr);
            let assistant_message = generated_request.into_assistant_message();
            request.messages.push(assistant_message);

            // Execute the HTTP request.
            debug!("sending proxy API request");
            let response = Client::new()
                .execute(http_request)
                .await
                .map_err(|err| ApiClientError::ApiConnection.into_response(&err))?;
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
