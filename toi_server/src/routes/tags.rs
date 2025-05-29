use axum::{extract::State, http::StatusCode, response::Json};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        state::ToiState,
        tags::{NewTag, NewTagRequest, Tag, TagSearchParams},
    },
    schema, utils,
};

const EDIT_SIMILARITY_THRESHOLD: f64 = 0.80;

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find tags similar to the one the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn tags_router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_tag))
        .routes(routes!(delete_matching_tags))
        .routes(routes!(get_matching_tags))
        .with_state(state)
}

pub async fn search_tags(
    state: &ToiState,
    params: TagSearchParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let TagSearchParams {
        ids,
        query,
        use_reranking_filter,
        use_edit_distance_filter,
        limit,
    } = params;

    let mut sql_query = schema::tags::table.select(Tag::as_select()).into_boxed();

    if let Some(ref query) = query {
        let input = EmbeddingPromptTemplate::builder()
            .instruction_prefix(INSTRUCTION_PREFIX.to_string())
            .query_prefix(QUERY_PREFIX.to_string())
            .build()
            .apply(query);
        let embedding_request = EmbeddingRequest { input };
        let embedding = state.model_client.embed(embedding_request).await?;
        sql_query = sql_query
            .filter(
                schema::tags::embedding
                    .cosine_distance(embedding.clone())
                    .le(state.server_config.distance_threshold),
            )
            .order(schema::tags::embedding.cosine_distance(embedding));
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        sql_query = sql_query.or_filter(schema::tags::id.eq_any(ids))
    }

    // Limit number of items.
    if let Some(limit) = limit {
        sql_query = sql_query.limit(limit);
    }

    // Get all the items that match the query.
    let tags: Vec<Tag> = sql_query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) =
        tags.into_iter().map(|tag| (tag.id, tag.name)).unzip();
    if ids.is_empty() {
        return Ok(ids);
    }

    // Rerank and filter items once more.
    let ids = match (query, use_reranking_filter) {
        (Some(query), Some(true)) => {
            let rerank_request = RerankRequest {
                query: query.clone(),
                documents,
            };
            let rerank_response = state.model_client.rerank(rerank_request).await?;
            rerank_response
                .results
                .into_iter()
                .filter(|item| {
                    let score = strsim::normalized_damerau_levenshtein(&query, &item.document.text);
                    let mut result =
                        item.relevance_score >= state.server_config.similarity_threshold;
                    if let Some(true) = use_edit_distance_filter {
                        result &= score >= EDIT_SIMILARITY_THRESHOLD;
                    }
                    result
                })
                .map(|item| ids[item.index])
                .collect()
        }
        _ => ids,
    };

    Ok(ids)
}

/// Add and return a tag.
///
/// Example queries for adding tags using this endpoint:
/// - Add a tag saying
/// - Add a tag that
/// - Keep tag on
/// - Remember that
/// - Make a tag
#[utoipa::path(
    post,
    path = "",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(NewTagRequest)))
    ),
    request_body = NewTagRequest,
    responses(
        (status = 201, description = "Successfully added a tag", body = Tag),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 409, description = "Tag already exists"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_tag(
    State(state): State<ToiState>,
    Json(params): Json<NewTagRequest>,
) -> Result<Json<Tag>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let NewTagRequest { name } = params;

    // Make sure a similar tag doesn't already exist.
    let params = TagSearchParams {
        ids: None,
        query: Some(name.clone()),
        use_reranking_filter: Some(true),
        use_edit_distance_filter: Some(true),
        limit: Some(1),
    };
    let ids = search_tags(&state, params, &mut conn).await;
    match ids {
        Ok(ids) if !ids.is_empty() => {
            return Err((StatusCode::CONFLICT, "tag already exists".to_string()));
        }
        Ok(_) => {}
        Err(other) => return Err(other),
    }

    let embedding_request = EmbeddingRequest {
        input: name.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let new_tag = NewTag { name, embedding };
    let result = diesel::insert_into(schema::tags::table)
        .values(new_tag)
        .returning(Tag::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(result))
}

/// Delete and return tags.
///
/// Example queries for deleting tags using this endpoint:
/// - Delete all tags
/// - Erase all tags
/// - Remove tags
/// - Delete as many tags
#[utoipa::path(
    post,
    path = "/delete",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(TagSearchParams)))
    ),
    request_body = TagSearchParams,
    responses(
        (status = 200, description = "Successfully deleted tags", body = [Tag]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No tags found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_tags(
    State(state): State<ToiState>,
    Json(params): Json<TagSearchParams>,
) -> Result<Json<Vec<Tag>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_tags(&state, params, &mut conn).await?;
    let tags = diesel::delete(schema::tags::table.filter(schema::tags::id.eq_any(ids)))
        .returning(Tag::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(tags))
}

/// Get tags.
///
/// Example queries for getting tags using this endpoint:
/// - Get all tags
/// - List all tags
/// - What tags are there
/// - How many tags are there
#[utoipa::path(
    post,
    path = "/search",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(TagSearchParams)))
    ),
    request_body = TagSearchParams,
    responses(
        (status = 200, description = "Successfully got tags", body = [Tag]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No tags found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_tags(
    State(state): State<ToiState>,
    Json(params): Json<TagSearchParams>,
) -> Result<Json<Vec<Tag>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_tags(&state, params, &mut conn).await?;
    let tags = schema::tags::table
        .select(Tag::as_select())
        .filter(schema::tags::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(tags))
}
