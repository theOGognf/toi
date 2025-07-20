use axum::{extract::State, http::StatusCode, response::Json};
use diesel::{BoolExpressionMethods, ExpressionMethods, JoinOnDsl, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use schemars::schema_for;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        attendees::{Attendee, AttendeeSearchParams, Attendees},
        contacts::{Contact, ContactSearchParams},
        events::{Event, EventSearchParams},
        state::ToiState,
    },
    routes::{contacts::search_contacts, events::search_events},
    schema, utils,
};

pub fn attendees_router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_attendees))
        .routes(routes!(delete_matching_attendees))
        .routes(routes!(get_matching_attendees))
        .with_state(state)
}

pub async fn search_attendees(
    state: &ToiState,
    params: AttendeeSearchParams,
    conn: &mut utils::Conn<'_>,
) -> Result<(Event, Vec<i32>), (StatusCode, String)> {
    let AttendeeSearchParams {
        event_id,
        event_query,
        event_use_reranking_filter,
        event_created_from,
        event_created_to,
        event_day,
        event_day_falls_on,
        event_order_by,
        contact_ids,
        contact_query,
        contact_use_reranking_filter,
        contact_limit,
    } = params;
    let event_query_params = EventSearchParams {
        ids: event_id.map(|i| vec![i]),
        event_day,
        event_day_falls_on,
        query: event_query,
        use_reranking_filter: event_use_reranking_filter,
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
    let contact_query_params = ContactSearchParams {
        ids: contact_ids,
        birthday: None,
        birthday_falls_on: None,
        query: contact_query,
        use_reranking_filter: contact_use_reranking_filter,
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
    extensions(
        ("x-json-schema-body" = json!(schema_for!(AttendeeSearchParams)))
    ),
    request_body = AttendeeSearchParams,
    responses(
        (status = 201, description = "Successfully added a attendee", body = Attendees),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
async fn add_attendees(
    State(state): State<ToiState>,
    Json(params): Json<AttendeeSearchParams>,
) -> Result<Json<Attendees>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (event, contact_ids) = search_attendees(&state, params, &mut conn).await?;
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
    post,
    path = "/delete",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(AttendeeSearchParams)))
    ),
    request_body = AttendeeSearchParams,
    responses(
        (status = 200, description = "Successfully deleted attendees", body = Attendees),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No event or contacts found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
async fn delete_matching_attendees(
    State(state): State<ToiState>,
    Json(params): Json<AttendeeSearchParams>,
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
    post,
    path = "/search",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(AttendeeSearchParams)))
    ),
    request_body = AttendeeSearchParams,
    responses(
        (status = 200, description = "Successfully got attendees", body = Attendees),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No event or contacts found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
async fn get_matching_attendees(
    State(state): State<ToiState>,
    Json(params): Json<AttendeeSearchParams>,
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
