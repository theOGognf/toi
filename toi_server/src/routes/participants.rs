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
        contacts::Contact,
        events::{Event, EventQueryParams, NewEvent, NewEventRequest},
        participants::{Participant, ParticipantQueryParams, Participants},
        state::ToiState,
    },
    schema, utils,
};

use super::{contacts::search_contacts, events::search_events};

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(
            add_participants,
            delete_matching_participants,
            get_matching_participants,
        ))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /participants extensions
    let add_participant_json_schema = schema_for!(NewEventRequest);
    let add_participant_json_schema =
        serde_json::to_value(add_participant_json_schema).expect("schema unserializable");
    let add_participant_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", add_participant_json_schema)
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(add_participant_extensions);

    // Update DELETE and GET /participants extensions
    let participants_json_schema = schema_for!(EventQueryParams);
    let participants_json_schema =
        serde_json::to_value(participants_json_schema).expect("schema unserializable");
    let participants_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", participants_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(participants_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(participants_extensions);

    router
}

pub async fn search_participants(
    state: &ToiState,
    params: ParticipantQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<(Event, Vec<Contact>), (StatusCode, String)> {
    let ParticipantQueryParams {
        event_query_params,
        contact_query_params,
    } = params;
    event_query_params.limit = Some(1);
    let event_id = search_events(state, &event_query_params, conn)
        .await?
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "Event not found".to_string()))?;
    let event = schema::events::table
        .select(Event::as_select())
        .filter(schema::events::id.eq(event_id))
        .first(conn)
        .await
        .map_err(utils::diesel_error)?;
    let contact_ids = search_contacts(state, &contact_query_params, conn).await?;
    let contacts = schema::contacts::table
        .select(Contact::as_select())
        .filter(schema::contacts::id.eq_any(contact_ids))
        .load(conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok((event, contacts))
}

/// Add participants.
///
/// Adds and returns the added participants' details.
///
/// Useful for answering phrases that start with the following:
/// - Add participant to...
/// - Remember this participant for...
/// - Make a participant for...
#[utoipa::path(
    post,
    path = "",
    request_body = ParticipantQueryParams,
    responses(
        (status = 201, description = "Successfully added a participant", body = Participants),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_participants(
    State(state): State<ToiState>,
    Json(body): Json<ParticipantQueryParams>,
) -> Result<Json<Participants>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (event, contacts) = search_participants(&state, body, &mut conn).await?;
    let new_participants: Vec<Participant> = contacts
        .iter()
        .map(|contact| Participant {
            event_id: event.id,
            contact_id: contact.id,
        })
        .collect();
    let _ = diesel::insert_into(schema::event_participants::table)
        .values(new_participants)
        .returning(Participant::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let participants = Participants { event, contacts };
    Ok(Json(participants))
}

/// Delete participants.
///
/// Remove and return participants that match a search criteria.
///
/// Useful for answering phrases that start with the following:
/// - Delete all participants with...
/// - Erase all participants that...
/// - Remove participants with...
/// - Delete participants...
#[utoipa::path(
    delete,
    path = "",
    params(ParticipantQueryParams),
    responses(
        (status = 200, description = "Successfully deleted participants", body = [Participants]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_participants(
    State(state): State<ToiState>,
    Query(params): Query<ParticipantQueryParams>,
) -> Result<Json<Participants>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (event, contacts) = search_participants(&state, params, &mut conn).await?;
    let contact_ids: Vec<i32> = contacts.iter().map(|contact| contact.id).collect();
    let _ = diesel::delete(schema::event_participants::table)
        .filter(
            schema::event_participants::event_id
                .eq(event.id)
                .and(schema::event_participants::contact_id.eq_any(contact_ids)),
        )
        .returning(Participant::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let participants = Participants { event, contacts };
    Ok(Json(participants))
}

/// Get participants.
///
/// Get participants that match a search criteria.
///
/// Useful for answering phrases that start with the following:
/// - Get all participants where...
/// - List all participants...
/// - What participants do I have on...
/// - How many participants do I have...
#[utoipa::path(
    get,
    path = "",
    params(EventQueryParams),
    responses(
        (status = 200, description = "Successfully got participants", body = [Event]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_participants(
    State(state): State<ToiState>,
    Query(params): Query<EventQueryParams>,
) -> Result<Json<Vec<Event>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (event, contacts) = search_participants(&state, params, &mut conn).await?;
    let contact_ids: Vec<i32> = contacts.iter().map(|contact| contact.id).collect();
    let _ = schema::event_participants::table
        .select()
        .filter(
            schema::event_participants::event_id
                .eq(event.id)
                .and(schema::event_participants::contact_id.eq_any(contact_ids)),
        )
        .returning(Participant::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let participants = Participants { event, contacts };
    Ok(Json(participants))
}
