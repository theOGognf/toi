use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{BoolExpressionMethods, ExpressionMethods, JoinOnDsl, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        attendees::{Attendee, AttendeeQueryParams, Attendees},
        contacts::{Contact, ContactQueryParams},
        events::{Event, EventQueryParams},
        search::SimilaritySearchParams,
        state::ToiState,
    },
    routes::{contacts::search_contacts, events::search_events},
    schema, utils,
};

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(
            add_attendees,
            delete_matching_attendees,
            get_matching_attendees,
        ))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /attendees extensions
    let attendee_json_schema = schema_for!(AttendeeQueryParams);
    let attendees_json_schema =
        serde_json::to_value(attendee_json_schema).expect("schema unserializable");
    let attendee_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", attendees_json_schema.clone())
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(attendee_extensions);

    // Update DELETE and GET /attendees extensions
    let attendees_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", attendees_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(attendees_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(attendees_extensions);

    router
}

pub async fn search_attendees(
    state: &ToiState,
    params: AttendeeQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<(Event, Vec<i32>), (StatusCode, String)> {
    let AttendeeQueryParams {
        event_id,
        event_query,
        event_use_reranking_filter,
        event_created_from,
        event_created_to,
        event_day_falls_on_search_params,
        event_order_by,
        contact_ids,
        contact_query,
        contact_use_reranking_filter,
        contact_limit,
    } = params;
    let event_query_params = EventQueryParams {
        ids: event_id.map(|i| vec![i]),
        event_day_falls_on_search_params,
        similarity_search_params: event_query.map(|query| SimilaritySearchParams {
            query,
            use_reranking_filter: event_use_reranking_filter,
        }),
        created_from: event_created_from,
        created_to: event_created_to,
        order_by: event_order_by,
        limit: Some(1),
    };
    let event_id = search_events(state, event_query_params, conn)
        .await?
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "event not found".to_string()))?;
    let event = schema::events::table
        .select(Event::as_select())
        .filter(schema::events::id.eq(event_id))
        .first(conn)
        .await
        .map_err(utils::diesel_error)?;
    let contact_query_params = ContactQueryParams {
        ids: contact_ids,
        birthday_falls_on_search_params: None,
        similarity_search_params: contact_query.map(|query| SimilaritySearchParams {
            query,
            use_reranking_filter: contact_use_reranking_filter,
        }),
        created_from: None,
        created_to: None,
        order_by: None,
        limit: contact_limit,
    };
    let contact_ids = search_contacts(state, contact_query_params, conn).await?;
    Ok((event, contact_ids))
}

/// Add and return attendees.
///
/// Example queries for adding attendees using this endpoint:
/// - Add attendee to
/// - Remember this attendee for
/// - Make a attendee for
#[utoipa::path(
    post,
    path = "",
    request_body = AttendeeQueryParams,
    responses(
        (status = 201, description = "Successfully added a attendee", body = Attendees),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_attendees(
    State(state): State<ToiState>,
    Json(body): Json<AttendeeQueryParams>,
) -> Result<Json<Attendees>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (event, contact_ids) = search_attendees(&state, body, &mut conn).await?;
    let contacts = schema::contacts::table
        .select(Contact::as_select())
        .filter(schema::contacts::id.eq_any(&contact_ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let new_attendees: Vec<Attendee> = contact_ids
        .iter()
        .map(|contact_id| Attendee {
            event_id: event.id,
            contact_id: *contact_id,
        })
        .collect();
    diesel::insert_into(schema::event_attendees::table)
        .values(new_attendees)
        .execute(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let attendees = Attendees { event, contacts };
    Ok(Json(attendees))
}

/// Delete and return attendees.
///
/// Example queries for deleting attendees using this endpoint:
/// - Delete all attendees with
/// - Erase all attendees that
/// - Remove attendees with
/// - Delete attendees
#[utoipa::path(
    delete,
    path = "",
    params(AttendeeQueryParams),
    responses(
        (status = 200, description = "Successfully deleted attendees", body = Attendees),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No event or contacts found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_attendees(
    State(state): State<ToiState>,
    Query(params): Query<AttendeeQueryParams>,
) -> Result<Json<Attendees>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (event, contact_ids) = search_attendees(&state, params, &mut conn).await?;
    let contacts = schema::contacts::table
        .select(Contact::as_select())
        .inner_join(
            schema::event_attendees::table
                .on(schema::event_attendees::contact_id.eq(schema::contacts::id)),
        )
        .filter(schema::event_attendees::event_id.eq(event.id))
        .filter(schema::contacts::id.eq_any(contact_ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let contact_ids: Vec<i32> = contacts.iter().map(|contact| contact.id).collect();
    diesel::delete(
        schema::event_attendees::table.filter(
            schema::event_attendees::event_id
                .eq(event.id)
                .and(schema::event_attendees::contact_id.eq_any(contact_ids)),
        ),
    )
    .execute(&mut conn)
    .await
    .map_err(utils::diesel_error)?;
    let attendees = Attendees { event, contacts };
    Ok(Json(attendees))
}

/// Get attendees.
///
/// Example queries for getting attendees using this endpoint:
/// - Get all attendees where
/// - List all attendees
/// - What attendees do I have on
/// - How many attendees do I have
#[utoipa::path(
    get,
    path = "",
    params(AttendeeQueryParams),
    responses(
        (status = 200, description = "Successfully got attendees", body = Attendees),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No event or contacts found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_attendees(
    State(state): State<ToiState>,
    Query(params): Query<AttendeeQueryParams>,
) -> Result<Json<Attendees>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (event, contact_ids) = search_attendees(&state, params, &mut conn).await?;
    let contacts = schema::contacts::table
        .select(Contact::as_select())
        .inner_join(
            schema::event_attendees::table
                .on(schema::event_attendees::contact_id.eq(schema::contacts::id)),
        )
        .filter(schema::event_attendees::event_id.eq(event.id))
        .filter(schema::contacts::id.eq_any(contact_ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let attendees = Attendees { event, contacts };
    Ok(Json(attendees))
}
