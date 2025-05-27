use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{BoolExpressionMethods, ExpressionMethods, JoinOnDsl, QueryDsl, SelectableHelper};
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        recipes::{
            NewRecipe, NewRecipeRequest, NewRecipeTag, NewRecipeTagsRequest, Recipe, RecipePreview,
            RecipeQueryParams, RecipeTagQueryParams, RecipeTags,
        },
        search::SimilaritySearchParams,
        state::ToiState,
        tags::{Tag, TagQueryParams},
    },
    routes::tags::search_tags,
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find recipes stored with details that the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn recipe_router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(
            add_recipe,
            delete_matching_recipes,
            get_matching_recipes,
        ))
        .routes(routes!(
            delete_matching_recipe_previews,
            get_matching_recipe_previews
        ))
        .routes(routes!(
            add_recipe_tags,
            delete_matching_recipe_tags,
            get_matching_recipe_tags
        ))
        .with_state(state)
}

pub async fn search_recipes(
    state: &ToiState,
    params: RecipeQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let RecipeQueryParams {
        ids,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        tags,
        limit,
    } = params;

    let mut query = schema::recipes::table
        .select(RecipePreview::as_select())
        .inner_join(
            schema::recipe_tags::table.on(schema::recipe_tags::recipe_id.eq(schema::recipes::id)),
        )
        .into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = created_from {
        query = query.filter(schema::recipes::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = created_to {
        query = query.filter(schema::recipes::created_at.le(created_to));
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::recipes::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::recipes::created_at.desc()),
        None => {
            // By default, filter items similar to a given query.
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
                        schema::recipes::embedding
                            .cosine_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::recipes::embedding.cosine_distance(embedding));
            }
        }
    }

    if let Some(tags) = tags {
        let mut tag_ids = vec![];
        for tag in tags {
            let params = TagQueryParams {
                ids: None,
                similarity_search_params: Some(SimilaritySearchParams {
                    query: tag,
                    use_reranking_filter: true,
                }),
                limit: Some(1),
            };
            let matching_tag_ids = search_tags(&state, params, conn).await?;
            let tag_id = matching_tag_ids
                .into_iter()
                .next()
                .expect("no matching tags");
            tag_ids.push(tag_id);
        }
        query = query.filter(schema::recipe_tags::tag_id.eq_any(tag_ids));
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        query = query.or_filter(schema::recipes::id.eq_any(ids))
    }

    // Limit number of items.
    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    // Get the item that matches the query.
    let recipe_previews: Vec<RecipePreview> =
        query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) = recipe_previews
        .into_iter()
        .map(|recipe| (recipe.id, recipe.description))
        .unzip();
    if ids.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no recipes found".to_string()));
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

pub async fn search_recipe_tags(
    state: &ToiState,
    params: RecipeTagQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<(RecipePreview, Vec<i32>), (StatusCode, String)> {
    let RecipeTagQueryParams {
        recipe_id,
        recipe_query,
        recipe_use_reranking_filter,
        recipe_created_from,
        recipe_created_to,
        recipe_order_by,
        tag_ids,
        tag_query,
        tag_use_reranking_filter,
        tag_limit,
    } = params;
    let recipe_query_params = RecipeQueryParams {
        ids: recipe_id.map(|i| vec![i]),
        similarity_search_params: recipe_query.map(|query| SimilaritySearchParams {
            query,
            use_reranking_filter: recipe_use_reranking_filter,
        }),
        created_from: recipe_created_from,
        created_to: recipe_created_to,
        order_by: recipe_order_by,
        tags: None,
        limit: Some(1),
    };
    let recipe_id = search_recipes(state, recipe_query_params, conn)
        .await?
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "recipe not found".to_string()))?;
    let recipe_preview = schema::recipes::table
        .select(RecipePreview::as_select())
        .filter(schema::recipes::id.eq(recipe_id))
        .first(conn)
        .await
        .map_err(utils::diesel_error)?;

    let mut query = schema::recipe_tags::table
        .select(schema::recipe_tags::tag_id)
        .filter(schema::recipe_tags::recipe_id.eq(recipe_preview.id))
        .into_boxed();

    if let Some(tag_ids) = tag_ids {
        query = query.filter(schema::recipe_tags::tag_id.eq_any(tag_ids));
    }

    let tag_ids = query.load(conn).await.map_err(utils::diesel_error)?;
    if tag_ids.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no tags found".to_string()));
    }

    let tag_query_params = TagQueryParams {
        ids: Some(tag_ids),
        similarity_search_params: tag_query.map(|query| SimilaritySearchParams {
            query,
            use_reranking_filter: tag_use_reranking_filter,
        }),
        limit: tag_limit,
    };
    let tag_ids = search_tags(state, tag_query_params, conn).await?;
    Ok((recipe_preview, tag_ids))
}

/// Add and return a recipe.
///
/// Example queries for adding a recipe using this endpoint:
/// - Add a recipe with
/// - Remember this recipe
/// - Make a recipe
#[utoipa::path(
    post,
    path = "",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(NewRecipeRequest)))
    ),
    request_body = NewRecipeRequest,
    responses(
        (status = 201, description = "Successfully added a recipe", body = Recipe),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_recipe(
    State(state): State<ToiState>,
    Json(body): Json<NewRecipeRequest>,
) -> Result<Json<Recipe>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let NewRecipeRequest {
        description,
        ingredients,
        instructions,
        tags,
    } = body;
    // Get tag IDs for matching tags.
    let mut tag_ids = vec![];
    for tag in tags {
        let params = TagQueryParams {
            ids: None,
            similarity_search_params: Some(SimilaritySearchParams {
                query: tag,
                use_reranking_filter: true,
            }),
            limit: Some(1),
        };
        let matching_tag_ids = search_tags(&state, params, &mut conn).await?;
        let tag_id = matching_tag_ids
            .into_iter()
            .next()
            .expect("no matching tags");
        tag_ids.push(tag_id);
    }
    // Get embedding for recipe description.
    let embedding_request = EmbeddingRequest {
        input: description.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    // Within a single transaction, add the recipe, and then add the recipe tags.
    let new_recipe = NewRecipe {
        description,
        ingredients,
        instructions,
        embedding,
    };
    let recipe = conn
        .transaction(|mut conn| {
            async move {
                // Insert the new recipe to get its database-generated ID.
                let recipe: Recipe = diesel::insert_into(schema::recipes::table)
                    .values(new_recipe)
                    .returning(Recipe::as_returning())
                    .get_result(&mut conn)
                    .await?;
                // Add the recipe tags using the recipe's database-generated ID.
                let new_recipe_tags: Vec<NewRecipeTag> = tag_ids
                    .into_iter()
                    .map(|tag_id| NewRecipeTag {
                        recipe_id: recipe.id,
                        tag_id,
                    })
                    .collect();
                diesel::insert_into(schema::recipe_tags::table)
                    .values(new_recipe_tags)
                    .execute(&mut conn)
                    .await?;
                Ok(recipe)
            }
            .scope_boxed()
        })
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(recipe))
}

/// Add recipe tags to existing recipes and return the updated recipes.
///
/// Example queries for adding recipe tags using this endpoint:
/// - Add recipe tags to
/// - Remember these recipe tags for
/// - Make recipe tags for
#[utoipa::path(
    post,
    path = "/tags",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(NewRecipeTagsRequest)))
    ),
    request_body = NewRecipeTagsRequest,
    responses(
        (status = 201, description = "Successfully added recipe tags", body = [Recipe]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_recipe_tags(
    State(state): State<ToiState>,
    Json(body): Json<NewRecipeTagsRequest>,
) -> Result<Json<Vec<Recipe>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let NewRecipeTagsRequest {
        ids,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        tags,
        limit,
    } = body;
    let params = RecipeQueryParams {
        ids,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        tags: None,
        limit,
    };
    let recipe_ids = search_recipes(&state, params, &mut conn).await?;
    // Get tag IDs for matching tags.
    let mut new_recipe_tags = vec![];
    for tag in tags {
        let params = TagQueryParams {
            ids: None,
            similarity_search_params: Some(SimilaritySearchParams {
                query: tag,
                use_reranking_filter: true,
            }),
            limit: Some(1),
        };
        let matching_tag_ids = search_tags(&state, params, &mut conn).await?;
        let tag_id = matching_tag_ids
            .into_iter()
            .next()
            .expect("no matching tags");
        for recipe_id in recipe_ids.iter() {
            let new_recipe_tag = NewRecipeTag {
                recipe_id: *recipe_id,
                tag_id,
            };
            new_recipe_tags.push(new_recipe_tag);
        }
    }
    diesel::insert_into(schema::recipe_tags::table)
        .values(new_recipe_tags)
        .execute(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let recipes = schema::recipes::table
        .select(Recipe::as_select())
        .filter(schema::recipes::id.eq_any(recipe_ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(recipes))
}

/// Delete and return recipes.
///
/// Example queries for deleting recipes using this endpoint:
/// - Delete recipes with
/// - Erase recipes that
/// - Remove recipes with
#[utoipa::path(
    delete,
    path = "",
    extensions(
        ("x-json-schema-params" = json!(schema_for!(RecipeQueryParams)))
    ),
    params(RecipeQueryParams),
    responses(
        (status = 200, description = "Successfully deleted recipes", body = [Recipe]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No recipes found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_recipes(
    State(state): State<ToiState>,
    Query(params): Query<RecipeQueryParams>,
) -> Result<Json<Vec<Recipe>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_recipes(&state, params, &mut conn).await?;
    let recipes = diesel::delete(schema::recipes::table.filter(schema::recipes::id.eq_any(ids)))
        .returning(Recipe::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(recipes))
}

/// Delete and return recipe previews.
///
/// Useful for deleting recipes in bulk.
///
/// Example queries for deleting recipe previews using this endpoint:
/// - Delete recipe previews with
/// - Erase recipe previews that
/// - Remove recipe previews with
#[utoipa::path(
    delete,
    path = "/previews",
    extensions(
        ("x-json-schema-params" = json!(schema_for!(RecipeQueryParams)))
    ),
    params(RecipeQueryParams),
    responses(
        (status = 200, description = "Successfully deleted recipe previews", body = [RecipePreview]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No recipe previews found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_recipe_previews(
    State(state): State<ToiState>,
    Query(params): Query<RecipeQueryParams>,
) -> Result<Json<Vec<RecipePreview>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_recipes(&state, params, &mut conn).await?;
    let recipe_previews =
        diesel::delete(schema::recipes::table.filter(schema::recipes::id.eq_any(ids)))
            .returning(RecipePreview::as_returning())
            .load(&mut conn)
            .await
            .map_err(utils::diesel_error)?;
    Ok(Json(recipe_previews))
}

/// Delete and return recipe tags.
///
/// Useful for deleting recipes in bulk.
///
/// Example queries for deleting recipe tags using this endpoint:
/// - Delete recipe tags for
/// - Erase recipe tags for
/// - Remove recipe tags with
#[utoipa::path(
    delete,
    path = "/tags",
    extensions(
        ("x-json-schema-params" = json!(schema_for!(RecipeTagQueryParams)))
    ),
    params(RecipeTagQueryParams),
    responses(
        (status = 200, description = "Successfully deleted recipe tags", body = RecipeTags),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No recipe or recipe tags found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_recipe_tags(
    State(state): State<ToiState>,
    Query(params): Query<RecipeTagQueryParams>,
) -> Result<Json<RecipeTags>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (recipe_preview, ids) = search_recipe_tags(&state, params, &mut conn).await?;
    let (recipe_preview, tags) = {
        conn.transaction(|mut conn| {
            async move {
                // Delete recipe tag items.
                let ids: Vec<i32> = diesel::delete(
                    schema::recipe_tags::table.filter(
                        schema::recipe_tags::recipe_id
                            .eq(recipe_preview.id)
                            .and(schema::recipe_tags::tag_id.eq_any(ids)),
                    ),
                )
                .returning(schema::recipe_tags::tag_id)
                .load(&mut conn)
                .await?;
                // Return the actual tag objects.
                let tags = schema::tags::table
                    .select(Tag::as_select())
                    .filter(schema::tags::id.eq_any(ids))
                    .load(&mut conn)
                    .await?;
                Ok((recipe_preview, tags))
            }
            .scope_boxed()
        })
        .await
        .map_err(utils::diesel_error)?
    };
    let recipe_tags = RecipeTags {
        recipe_preview,
        tags,
    };
    Ok(Json(recipe_tags))
}

/// Get recipes.
///
/// Example queries for getting recipes using this endpoint:
/// - Get recipes where
/// - List recipes
/// - What recipes do I have on
#[utoipa::path(
    get,
    path = "",
    extensions(
        ("x-json-schema-params" = json!(schema_for!(RecipeQueryParams)))
    ),
    params(RecipeQueryParams),
    responses(
        (status = 200, description = "Successfully got recipes", body = [Recipe]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No recipes found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_recipes(
    State(state): State<ToiState>,
    Query(params): Query<RecipeQueryParams>,
) -> Result<Json<Vec<Recipe>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_recipes(&state, params, &mut conn).await?;
    let recipes = schema::recipes::table
        .select(Recipe::as_select())
        .filter(schema::recipes::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(recipes))
}

/// Get recipe previews.
///
/// Useful for quickly searching through many recipes.
///
/// Example queries for getting recipe previews using this endpoint:
/// - Get recipe previews where
/// - List recipe previews
/// - What recipe previews do I have on
#[utoipa::path(
    get,
    path = "/previews",
    extensions(
        ("x-json-schema-params" = json!(schema_for!(RecipeQueryParams)))
    ),
    params(RecipeQueryParams),
    responses(
        (status = 200, description = "Successfully got recipe previews", body = [RecipePreview]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No recipe previews found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_recipe_previews(
    State(state): State<ToiState>,
    Query(params): Query<RecipeQueryParams>,
) -> Result<Json<Vec<RecipePreview>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_recipes(&state, params, &mut conn).await?;
    let recipe_previews = schema::recipes::table
        .select(RecipePreview::as_select())
        .filter(schema::recipes::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(recipe_previews))
}

/// Get recipe tags.
///
/// Example queries for getting recipe tags using this endpoint:
/// - Get recipe tags where
/// - List recipe tags
/// - What recipe tags do I have on
#[utoipa::path(
    get,
    path = "/tags",
    extensions(
        ("x-json-schema-params" = json!(schema_for!(RecipeTagQueryParams)))
    ),
    params(RecipeTagQueryParams),
    responses(
        (status = 200, description = "Successfully got recipe tags", body = RecipeTags),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No recipe or recipe tags found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_recipe_tags(
    State(state): State<ToiState>,
    Query(params): Query<RecipeTagQueryParams>,
) -> Result<Json<RecipeTags>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (recipe_preview, ids) = search_recipe_tags(&state, params, &mut conn).await?;
    let tags = schema::tags::table
        .select(Tag::as_select())
        .filter(schema::tags::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let recipe_tags = RecipeTags {
        recipe_preview,
        tags,
    };
    Ok(Json(recipe_tags))
}
