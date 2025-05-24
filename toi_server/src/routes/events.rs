use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{Datelike, Duration, Month, NaiveDate, NaiveTime};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        events::{Event, EventQueryParams, NewEvent, NewEventRequest},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find events stored with details that the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(
            add_event,
            delete_matching_events,
            get_matching_events,
        ))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /events extensions
    let add_event_json_schema = schema_for!(NewEventRequest);
    let add_event_json_schema =
        serde_json::to_value(add_event_json_schema).expect("schema unserializable");
    let add_event_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", add_event_json_schema)
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(add_event_extensions);

    // Update DELETE and GET /events extensions
    let events_json_schema = schema_for!(EventQueryParams);
    let events_json_schema =
        serde_json::to_value(events_json_schema).expect("schema unserializable");
    let events_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", events_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(events_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(events_extensions);

    router
}

pub async fn search_events(
    state: &ToiState,
    params: EventQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let EventQueryParams {
        ids,
        event_day_falls_on_search_params,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        limit,
    } = params;

    let mut query = schema::events::table
        .select(Event::as_select())
        .into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = created_from {
        query = query.filter(schema::events::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = created_to {
        query = query.filter(schema::events::created_at.le(created_to));
    }

    // Filter items according to event days.
    if let Some(event_day_falls_on_search_params) = event_day_falls_on_search_params {
        match event_day_falls_on_search_params.falls_on {
            utils::DateFallsOn::Month => {
                let year = event_day_falls_on_search_params.event_day.year();
                let month = event_day_falls_on_search_params.event_day.month();
                let num_days_in_month = {
                    let month = u8::try_from(month).map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "invalid event day search month".to_string(),
                        )
                    })?;
                    let month = Month::try_from(month).map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "invalid event day search month".to_string(),
                        )
                    })?;
                    month.num_days(year).ok_or((
                        StatusCode::BAD_REQUEST,
                        "invalid event day search year".to_string(),
                    ))?
                };
                let first_day_of_month = NaiveDate::from_ymd_opt(year, month, 1).ok_or((
                    StatusCode::BAD_REQUEST,
                    "invalid event day search".to_string(),
                ))?;
                let first_day_of_month_start_time =
                    first_day_of_month.and_time(NaiveTime::default()).and_utc();
                let last_day_of_month =
                    NaiveDate::from_ymd_opt(year, month, num_days_in_month.into()).ok_or((
                        StatusCode::BAD_REQUEST,
                        "invalid event day search".to_string(),
                    ))?;
                let end_time = NaiveTime::from_hms_opt(23, 59, 59).expect("invalid time");
                let last_day_of_month_end_time = last_day_of_month.and_time(end_time).and_utc();
                query = query.filter(
                    (schema::events::starts_at
                        .ge(first_day_of_month_start_time)
                        .and(schema::events::starts_at.le(last_day_of_month_end_time)))
                    .or(schema::events::ends_at
                        .ge(first_day_of_month_start_time)
                        .and(schema::events::ends_at.le(last_day_of_month_end_time))),
                );
            }
            utils::DateFallsOn::Week => {
                let num_days_from_sunday = event_day_falls_on_search_params
                    .event_day
                    .weekday()
                    .num_days_from_sunday();
                let this_weeks_sunday = event_day_falls_on_search_params.event_day
                    - Duration::days(num_days_from_sunday.into());
                let this_weeks_start_time =
                    this_weeks_sunday.and_time(NaiveTime::default()).and_utc();
                let end_time = NaiveTime::from_hms_opt(23, 59, 59).expect("invalid time");
                let this_weeks_saturday = this_weeks_sunday + Duration::days(6);
                let this_weeks_end_time = this_weeks_saturday.and_time(end_time).and_utc();
                query = query.filter(
                    (schema::events::starts_at
                        .ge(this_weeks_start_time)
                        .and(schema::events::starts_at.le(this_weeks_end_time)))
                    .or(schema::events::ends_at
                        .ge(this_weeks_start_time)
                        .and(schema::events::ends_at.le(this_weeks_end_time))),
                );
            }
            utils::DateFallsOn::Day => {
                let day_of_event_start_time = event_day_falls_on_search_params
                    .event_day
                    .and_time(NaiveTime::default())
                    .and_utc();
                let end_time = NaiveTime::from_hms_opt(23, 59, 59).expect("invalid time");
                let day_of_event_end_time = event_day_falls_on_search_params
                    .event_day
                    .and_time(end_time)
                    .and_utc();
                query = query.filter(
                    (schema::events::starts_at
                        .ge(day_of_event_start_time)
                        .and(schema::events::starts_at.le(day_of_event_end_time)))
                    .or(schema::events::ends_at
                        .ge(day_of_event_start_time)
                        .and(schema::events::ends_at.le(day_of_event_end_time))),
                );
            }
        }
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::events::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::events::created_at.desc()),
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
                        schema::events::embedding
                            .cosine_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::events::embedding.cosine_distance(embedding));
            }
        }
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        query = query.or_filter(schema::events::id.eq_any(ids))
    }

    // Limit number of items.
    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    // Get all the items that match the query.
    let events: Vec<Event> = query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) = events
        .into_iter()
        .map(|event| (event.id, event.description))
        .unzip();
    if ids.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no events found".to_string()));
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

/// Add and return an event.
///
/// Example queries for adding an event using this endpoint:
/// - Add an event with
/// - Remember this event
/// - Make an event
#[utoipa::path(
    post,
    path = "",
    request_body = NewEventRequest,
    responses(
        (status = 201, description = "Successfully added an event", body = Event),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_event(
    State(state): State<ToiState>,
    Json(body): Json<NewEventRequest>,
) -> Result<Json<Event>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let embedding_request = EmbeddingRequest {
        input: body.description.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let NewEventRequest {
        description,
        starts_at,
        ends_at,
    } = body;
    let new_event = NewEvent {
        description,
        embedding,
        starts_at,
        ends_at,
    };
    let res = diesel::insert_into(schema::events::table)
        .values(new_event)
        .returning(Event::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete and return events.
///
/// Example queries for deleting events using this endpoint:
/// - Delete all events with
/// - Erase all events that
/// - Remove events with
/// - Delete events
#[utoipa::path(
    delete,
    path = "",
    params(EventQueryParams),
    responses(
        (status = 200, description = "Successfully deleted events", body = [Event]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No events found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_events(
    State(state): State<ToiState>,
    Query(params): Query<EventQueryParams>,
) -> Result<Json<Vec<Event>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_events(&state, params, &mut conn).await?;
    let events = diesel::delete(schema::events::table.filter(schema::events::id.eq_any(ids)))
        .returning(Event::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(events))
}

/// Get events.
///
/// Example queries for getting events using this endpoint:
/// - Get all events where
/// - List all events
/// - What events do I have on
/// - How many events do I have
#[utoipa::path(
    get,
    path = "",
    params(EventQueryParams),
    responses(
        (status = 200, description = "Successfully got events", body = [Event]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No events found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_events(
    State(state): State<ToiState>,
    Query(params): Query<EventQueryParams>,
) -> Result<Json<Vec<Event>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_events(&state, params, &mut conn).await?;
    let events = schema::events::table
        .select(Event::as_select())
        .filter(schema::events::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(events))
}
