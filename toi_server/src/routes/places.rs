use axum::{extract::State, http::StatusCode, response::Json};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        places::{NewPlace, NewPlaceRequest, Place, PlaceSearchParams, UpdatePlaceRequest},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find places stored with details that the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn places_router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_place, update_matching_place))
        .routes(routes!(delete_matching_places,))
        .routes(routes!(get_matching_places))
        .with_state(state)
}

pub async fn search_places(
    state: &ToiState,
    params: PlaceSearchParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let PlaceSearchParams {
        ids,
        query,
        use_reranking_filter,
        created_from,
        created_to,
        order_by,
        limit,
    } = params;

    let mut sql_query = schema::places::table
        .select(Place::as_select())
        .into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = created_from {
        sql_query = sql_query.filter(schema::places::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = created_to {
        sql_query = sql_query.filter(schema::places::created_at.le(created_to));
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => sql_query = sql_query.order(schema::places::created_at),
        Some(utils::OrderBy::Newest) => {
            sql_query = sql_query.order(schema::places::created_at.desc());
        }
        None => {
            // By default, filter items similar to a given query.
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
                        schema::places::embedding
                            .cosine_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::places::embedding.cosine_distance(embedding));
            }
        }
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        sql_query = sql_query.or_filter(schema::places::id.eq_any(ids));
    }

    // Limit number of items.
    if let Some(limit) = limit {
        sql_query = sql_query.limit(limit);
    }

    // Get all the items that match the query.
    let places: Vec<Place> = sql_query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) = places
        .into_iter()
        .map(|place| {
            let Place {
                id,
                name,
                description,
                address,
                phone,
                ..
            } = place;
            let new_place_request = NewPlaceRequest {
                name,
                description,
                address,
                phone,
            };
            (id, new_place_request.to_string())
        })
        .unzip();
    if ids.is_empty() {
        return Ok(ids);
    }

    // Rerank and filter items once more.
    let ids = match (query, use_reranking_filter) {
        (Some(query), Some(true)) => {
            let rerank_request = RerankRequest { query, documents };
            let rerank_response = state.model_client.rerank(rerank_request).await?;
            rerank_response
                .results
                .into_iter()
                .filter(|item| item.relevance_score >= state.server_config.similarity_threshold)
                .map(|item| ids[item.index])
                .collect()
        }
        _ => ids,
    };

    Ok(ids)
}

/// Add and return a place.
///
/// Example queries for adding places using this endpoint:
/// - Add a place
/// - Remember this place
/// - Make a place
#[utoipa::path(
    post,
    path = "",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(NewPlaceRequest)))
    ),
    request_body = NewPlaceRequest,
    responses(
        (status = 201, description = "Successfully added a place", body = Place),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_place(
    State(state): State<ToiState>,
    Json(params): Json<NewPlaceRequest>,
) -> Result<Json<Place>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let embedding_request = EmbeddingRequest {
        input: params.to_string(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let NewPlaceRequest {
        name,
        description,
        address,
        phone,
    } = params;
    let new_place = NewPlace {
        name,
        description,
        address,
        phone,
        embedding,
    };
    let result = diesel::insert_into(schema::places::table)
        .values(new_place)
        .returning(Place::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(result))
}

/// Delete and return places.
///
/// Example queries for deleting places using this endpoint:
/// - Delete all places
/// - Erase all places
/// - Remove places
#[utoipa::path(
    post,
    path = "/delete",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(PlaceSearchParams)))
    ),
    request_body = PlaceSearchParams,
    responses(
        (status = 200, description = "Successfully deleted places", body = [Place]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No places found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_places(
    State(state): State<ToiState>,
    Json(params): Json<PlaceSearchParams>,
) -> Result<Json<Vec<Place>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_places(&state, params, &mut conn).await?;
    let places = diesel::delete(schema::places::table.filter(schema::places::id.eq_any(ids)))
        .returning(Place::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(places))
}

/// Get places.
///
/// Example queries for getting places using this endpoint:
/// - Get all places
/// - List all places
/// - What places
/// - How many places
#[utoipa::path(
    post,
    path = "/search",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(PlaceSearchParams)))
    ),
    request_body = PlaceSearchParams,
    responses(
        (status = 200, description = "Successfully got places", body = [Place]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No places found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_places(
    State(state): State<ToiState>,
    Json(params): Json<PlaceSearchParams>,
) -> Result<Json<Vec<Place>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_places(&state, params, &mut conn).await?;
    let places = schema::places::table
        .select(Place::as_select())
        .filter(schema::places::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(places))
}

/// Update and return a place.
///
/// Example queries for updating a place using this endpoint:
/// - Update my place
/// - Update the address
/// - Remember this about this place
#[utoipa::path(
    put,
    path = "",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(UpdatePlaceRequest)))
    ),
    request_body = UpdatePlaceRequest,
    responses(
        (status = 200, description = "Successfully updated place", body = Place),
        (status = 404, description = "Place not found")
    )
)]
#[axum::debug_handler]
pub async fn update_matching_place(
    State(state): State<ToiState>,
    Json(params): Json<UpdatePlaceRequest>,
) -> Result<Json<Place>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let UpdatePlaceRequest {
        id,
        place_updates,
        query,
        use_reranking_filter,
        created_from,
        created_to,
        order_by,
    } = params;
    let params = PlaceSearchParams {
        ids: id.map(|i| vec![i]),
        query,
        use_reranking_filter,
        created_from,
        created_to,
        order_by,
        limit: Some(1),
    };
    let id = search_places(&state, params, &mut conn)
        .await?
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "place not found".to_string()))?;
    let mut place = schema::places::table
        .select(Place::as_select())
        .filter(schema::places::id.eq(id))
        .first(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    place.update(place_updates);
    let Place {
        id,
        name,
        description,
        address,
        phone,
        ..
    } = place;
    let new_place_request = NewPlaceRequest {
        name,
        description,
        address,
        phone,
    };
    let embedding_request = EmbeddingRequest {
        input: new_place_request.to_string(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let NewPlaceRequest {
        name,
        description,
        address,
        phone,
    } = new_place_request;
    let new_place = NewPlace {
        name,
        description,
        address,
        phone,
        embedding,
    };
    let place = diesel::update(schema::places::table.filter(schema::places::id.eq(id)))
        .set(&new_place)
        .returning(Place::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(place))
}
