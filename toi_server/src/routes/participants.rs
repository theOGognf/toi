use std::collections::HashSet;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        contacts::{Contact, ContactQueryParams},
        events::{Event, EventQueryParams},
        participants::{Participant, ParticipantQueryParams, Participants},
        search::SimilaritySearchParams,
        state::ToiState,
    },
    routes::{contacts::search_contacts, events::search_events},
    schema, utils,
};

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
    let participant_json_schema = schema_for!(ParticipantQueryParams);
    let participants_json_schema =
        serde_json::to_value(participant_json_schema).expect("schema unserializable");
    let participant_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", participants_json_schema.clone())
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(participant_extensions);

    // Update DELETE and GET /participants extensions
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
        event_query,
        event_distance_threshold,
        event_similarity_threshold,
        event_created_from,
        event_created_to,
        event_starts_from,
        event_starts_to,
        event_ends_from,
        event_ends_to,
        event_order_by,
        contact_query,
        contact_distance_threshold,
        contact_similarity_threshold,
        contact_limit,
    } = params;
    let event_query_params = EventQueryParams {
        event_day_falls_on_search_params: None,
        similarity_search_params: Some(SimilaritySearchParams {
            query: event_query,
            distance_threshold: event_distance_threshold,
            similarity_threshold: event_similarity_threshold,
        }),
        created_from: event_created_from,
        created_to: event_created_to,
        starts_from: event_starts_from,
        starts_to: event_starts_to,
        ends_from: event_ends_from,
        ends_to: event_ends_to,
        order_by: event_order_by,
        limit: Some(1),
    };
    let event_id = search_events(state, &event_query_params, conn)
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
        birthday_falls_on_search_params: None,
        similarity_search_params: Some(SimilaritySearchParams {
            query: contact_query,
            distance_threshold: contact_distance_threshold,
            similarity_threshold: contact_similarity_threshold,
        }),
        created_from: None,
        created_to: None,
        order_by: None,
        limit: contact_limit,
    };
    let contact_ids = search_contacts(state, &contact_query_params, conn).await?;
    let contacts = schema::contacts::table
        .select(Contact::as_select())
        .filter(schema::contacts::id.eq_any(contact_ids))
        .load(conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok((event, contacts))
}

/// Add and return participants.
///
/// Example queries for adding participants using this endpoint:
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

/// Delete and return participants.
///
/// Example queries for deleting participants using this endpoint:
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
    let participants = diesel::delete(schema::event_participants::table)
        .filter(
            schema::event_participants::event_id
                .eq(event.id)
                .and(schema::event_participants::contact_id.eq_any(contact_ids)),
        )
        .returning(schema::event_participants::contact_id)
        .get_results(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let participants: HashSet<i32> = HashSet::from_iter(participants);
    let participants = Participants {
        event,
        contacts: contacts
            .into_iter()
            .filter(|contact| participants.contains(&contact.id))
            .collect(),
    };
    Ok(Json(participants))
}

/// Get participants.
///
/// Example queries for getting participants using this endpoint:
/// - Get all participants where...
/// - List all participants...
/// - What participants do I have on...
/// - How many participants do I have...
#[utoipa::path(
    get,
    path = "",
    params(ParticipantQueryParams),
    responses(
        (status = 200, description = "Successfully got participants", body = [Participants]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_participants(
    State(state): State<ToiState>,
    Query(params): Query<ParticipantQueryParams>,
) -> Result<Json<Participants>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (event, contacts) = search_participants(&state, params, &mut conn).await?;
    let contact_ids: Vec<i32> = contacts.iter().map(|contact| contact.id).collect();
    let participants = schema::event_participants::table
        .select(schema::event_participants::contact_id)
        .filter(
            schema::event_participants::event_id
                .eq(event.id)
                .and(schema::event_participants::contact_id.eq_any(contact_ids)),
        )
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let participants: HashSet<i32> = HashSet::from_iter(participants);
    let participants = Participants {
        event,
        contacts: contacts
            .into_iter()
            .filter(|contact| participants.contains(&contact.id))
            .collect(),
    };
    Ok(Json(participants))
}
