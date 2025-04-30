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
        client::{EmbeddingPromptTemplate, EmbeddingRequest},
        notes::{NewNote, NewNoteRequest, Note, NoteQueryParams},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find similar notes to the one the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(add_note))
        .routes(routes!(delete_matching_notes, get_matching_notes))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = &mut openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /notes extensions
    let add_note_json_schema = schema_for!(NewNoteRequest);
    let add_note_json_schema =
        serde_json::to_value(add_note_json_schema).expect("schema unserializable");
    let add_note_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", add_note_json_schema)
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(add_note_extensions);

    // Update DELETE and GET /notes extensions
    let notes_json_schema = schema_for!(NoteQueryParams);
    let notes_json_schema = serde_json::to_value(notes_json_schema).expect("schema unserializable");
    let notes_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", notes_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(notes_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(notes_extensions);

    router
}

/// Add a note.
///
/// Useful for answering phrases that start with the following:
/// - Add a note saying...
/// - Add a note that...
/// - Keep note on...
/// - Remember that...
/// - Make a note...
#[utoipa::path(
    post,
    path = "",
    request_body = NewNoteRequest,
    responses(
        (status = 201, description = "Successfully added a note", body = Note),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_note(
    State(state): State<ToiState>,
    Json(body): Json<NewNoteRequest>,
) -> Result<Json<Note>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let embedding_request = EmbeddingRequest {
        input: body.content.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let new_note = NewNote {
        content: body.content,
        embedding,
    };
    let res = diesel::insert_into(schema::notes::table)
        .values(new_note)
        .returning(Note::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete notes.
///
/// Delete notes that match a search criteria. Useful for deleting notes in bulk.
///
/// Useful for answering phrases that start with the following:
/// - Delete all notes related to...
/// - Erase all notes about...
/// - Remove notes there are on...
/// - Delete as many notes there are about...
#[utoipa::path(
    delete,
    path = "",
    params(NoteQueryParams),
    responses(
        (status = 200, description = "Successfully deleted notes", body = [Note]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_notes(
    State(state): State<ToiState>,
    Query(params): Query<NoteQueryParams>,
) -> Result<Json<Vec<Note>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let mut query = schema::notes::table.select(schema::notes::id).into_boxed();

    // Filter notes similar to a query.
    if let Some(note_similarity_search_params) = params.similarity_search_params {
        let input = EmbeddingPromptTemplate::builder()
            .instruction_prefix(INSTRUCTION_PREFIX.to_string())
            .query_prefix(QUERY_PREFIX.to_string())
            .build()
            .apply(note_similarity_search_params.query);
        let embedding_request = EmbeddingRequest { input };
        let embedding = state.model_client.embed(embedding_request).await?;
        query = query.filter(
            schema::notes::embedding
                .cosine_distance(embedding.clone())
                .le(note_similarity_search_params.distance_threshold),
        );

        // Sort by relevance.
        if params.order_by == Some(utils::OrderBy::Relevance) {
            query = query.order(schema::notes::embedding.l2_distance(embedding));
        }
    }

    // Filter notes created on or after date.
    if let Some(created_from) = params.created_from {
        query = query.filter(schema::notes::created_at.ge(created_from));
    }

    // Filter notes created on or before date.
    if let Some(created_to) = params.created_to {
        query = query.filter(schema::notes::created_at.le(created_to));
    }

    match params.order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::notes::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::notes::created_at.desc()),
        _ => {}
    }

    // Limit number of notes deleted.
    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    let res = diesel::delete(schema::notes::table.filter(schema::notes::id.eq_any(query)))
        .returning(Note::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::internal_error)?;
    Ok(Json(res))
}

/// Get notes.
///
/// Get notes that match a search criteria. Useful for getting notes in bulk.
///
/// Useful for answering phrases that start with the following:
/// - Get all notes related to...
/// - List all notes about...
/// - What notes are there on...
/// - How many notes are there about...
#[utoipa::path(
    get,
    path = "",
    params(NoteQueryParams),
    responses(
        (status = 200, description = "Successfully got notes", body = [Note]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_notes(
    State(state): State<ToiState>,
    Query(params): Query<NoteQueryParams>,
) -> Result<Json<Vec<Note>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let mut query = schema::notes::table.select(Note::as_select()).into_boxed();

    // Filter notes similar to a query.
    if let Some(note_similarity_search_params) = params.similarity_search_params {
        let input = EmbeddingPromptTemplate::builder()
            .instruction_prefix(INSTRUCTION_PREFIX.to_string())
            .query_prefix(QUERY_PREFIX.to_string())
            .build()
            .apply(note_similarity_search_params.query);
        let embedding_request = EmbeddingRequest { input };
        let embedding = state.model_client.embed(embedding_request).await?;
        query = query.filter(
            schema::notes::embedding
                .cosine_distance(embedding.clone())
                .le(note_similarity_search_params.distance_threshold),
        );

        // Sort by relevance.
        if params.order_by == Some(utils::OrderBy::Relevance) {
            query = query.order(schema::notes::embedding.l2_distance(embedding));
        }
    }

    // Filter notes created on or after date.
    if let Some(created_from) = params.created_from {
        query = query.filter(schema::notes::created_at.ge(created_from));
    }

    // Filter notes created on or before date.
    if let Some(created_to) = params.created_to {
        query = query.filter(schema::notes::created_at.le(created_to));
    }

    match params.order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::notes::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::notes::created_at.desc()),
        _ => {}
    }

    // Limit number of notes returned.
    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    let res = query.load(&mut conn).await.map_err(utils::internal_error)?;
    Ok(Json(res))
}
