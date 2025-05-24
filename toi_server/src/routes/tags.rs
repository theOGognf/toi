use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        state::ToiState,
        tags::{NewTag, NewTagRequest, Tag, TagQueryParams},
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find tags similar to the one the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(add_tag, delete_matching_tags, get_matching_tags))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /tags extensions
    let add_tag_json_schema = schema_for!(NewTagRequest);
    let add_tag_json_schema =
        serde_json::to_value(add_tag_json_schema).expect("schema unserializable");
    let add_tag_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", add_tag_json_schema)
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(add_tag_extensions);

    // Update DELETE and GET /tags extensions
    let tags_json_schema = schema_for!(TagQueryParams);
    let tags_json_schema = serde_json::to_value(tags_json_schema).expect("schema unserializable");
    let tags_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", tags_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(tags_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(tags_extensions);

    router
}

async fn search_tags(
    state: &ToiState,
    params: TagQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let TagQueryParams {
        ids,
        similarity_search_params,
        limit,
    } = params;

    let mut query = schema::tags::table.select(Tag::as_select()).into_boxed();

    if let Some(ref similarity_search_params) = similarity_search_params {
        let input = EmbeddingPromptTemplate::builder()
            .instruction_prefix(INSTRUCTION_PREFIX.to_string())
            .query_prefix(QUERY_PREFIX.to_string())
            .build()
            .apply(&similarity_search_params.query);
        let embedding_request = EmbeddingRequest { input };
        let embedding = state.model_client.embed(embedding_request).await?;
        query = query
            .filter(
                schema::tags::embedding
                    .cosine_distance(embedding.clone())
                    .le(state.server_config.distance_threshold),
            )
            .order(schema::tags::embedding.cosine_distance(embedding));
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        query = query.or_filter(schema::tags::id.eq_any(ids))
    }

    // Limit number of items.
    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    // Get all the items that match the query.
    let tags: Vec<Tag> = query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) =
        tags.into_iter().map(|tag| (tag.id, tag.name)).unzip();
    if ids.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no tags found".to_string()));
    }

    // Rerank and filter items once more.
    let ids = match similarity_search_params {
        Some(similarity_search_params) => {
            if similarity_search_params.use_reranking_filter {
                let rerank_request = RerankRequest {
                    query: similarity_search_params.query,
                    documents,
                };
                let rerank_response = state.model_client.rerank(rerank_request).await?;
                rerank_response
                    .results
                    .into_iter()
                    .filter(|item| item.relevance_score >= state.server_config.similarity_threshold)
                    .map(|item| ids[item.index])
                    .collect()
            } else {
                ids
            }
        }
        None => ids,
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
    Json(body): Json<NewTagRequest>,
) -> Result<Json<Tag>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let NewTagRequest { name } = body;

    let input = EmbeddingPromptTemplate::builder()
        .instruction_prefix(INSTRUCTION_PREFIX.to_string())
        .query_prefix(QUERY_PREFIX.to_string())
        .build()
        .apply(&name);
    let embedding_request = EmbeddingRequest { input };
    let embedding = state.model_client.embed(embedding_request).await?;

    // Make sure a similar tag doesn't already exist.
    let tags: Vec<Tag> = schema::tags::table
        .select(Tag::as_select())
        .filter(
            schema::tags::embedding
                .cosine_distance(embedding.clone())
                .le(state.server_config.distance_threshold),
        )
        .order((
            schema::tags::embedding.cosine_distance(embedding.clone()),
            schema::tags::id,
        ))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    if !tags.is_empty() {
        let (ids, documents): (Vec<i32>, Vec<String>) =
            tags.into_iter().map(|tag| (tag.id, tag.name)).unzip();

        let rerank_request = RerankRequest {
            query: name.clone(),
            documents,
        };
        let rerank_response = state.model_client.rerank(rerank_request).await?;
        let ids: Vec<i32> = rerank_response
            .results
            .into_iter()
            .filter(|item| item.relevance_score >= state.server_config.similarity_threshold)
            .map(|item| ids[item.index])
            .collect();
        if !ids.is_empty() {
            return Err((StatusCode::CONFLICT, "tag already exists".to_string()));
        }
    }

    let new_tag = NewTag { name, embedding };
    let res = diesel::insert_into(schema::tags::table)
        .values(new_tag)
        .returning(Tag::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete and return tags.
///
/// Example queries for deleting tags using this endpoint:
/// - Delete all tags
/// - Erase all tags
/// - Remove tags
/// - Delete as many tags
#[utoipa::path(
    delete,
    path = "",
    params(TagQueryParams),
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
    Query(params): Query<TagQueryParams>,
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
    get,
    path = "",
    params(TagQueryParams),
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
    Query(params): Query<TagQueryParams>,
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
