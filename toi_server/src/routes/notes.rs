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
        notes::{NewNote, NewNoteRequest, Note, NoteQueryParams},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find notes similar to the one the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn notes_router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(add_note, delete_matching_notes, get_matching_notes))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

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

async fn search_notes(
    state: &ToiState,
    params: NoteQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let NoteQueryParams {
        ids,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        limit,
    } = params;

    let mut query = schema::notes::table.select(Note::as_select()).into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = created_from {
        query = query.filter(schema::notes::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = created_to {
        query = query.filter(schema::notes::created_at.le(created_to));
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::notes::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::notes::created_at.desc()),
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
                        schema::notes::embedding
                            .cosine_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::notes::embedding.cosine_distance(embedding));
            }
        }
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        query = query.or_filter(schema::notes::id.eq_any(ids))
    }

    // Limit number of items.
    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    // Get all the items that match the query.
    let notes: Vec<Note> = query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) = notes
        .into_iter()
        .map(|note| (note.id, note.content))
        .unzip();
    if ids.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no notes found".to_string()));
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

/// Add and return a note.
///
/// Example queries for adding notes using this endpoint:
/// - Add a note saying
/// - Add a note that
/// - Keep note on
/// - Remember that
/// - Make a note
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
    let NewNoteRequest { content } = body;
    let embedding_request = EmbeddingRequest {
        input: content.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let new_note = NewNote { content, embedding };
    let result = diesel::insert_into(schema::notes::table)
        .values(new_note)
        .returning(Note::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(result))
}

/// Delete and return notes.
///
/// Example queries for deleting notes using this endpoint:
/// - Delete all notes
/// - Erase all notes
/// - Remove notes
/// - Delete as many notes
#[utoipa::path(
    delete,
    path = "",
    params(NoteQueryParams),
    responses(
        (status = 200, description = "Successfully deleted notes", body = [Note]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No notes found"),
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
    let ids = search_notes(&state, params, &mut conn).await?;
    let notes = diesel::delete(schema::notes::table.filter(schema::notes::id.eq_any(ids)))
        .returning(Note::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(notes))
}

/// Get notes.
///
/// Example queries for getting notes using this endpoint:
/// - Get all notes
/// - List all notes
/// - What notes are there
/// - How many notes are there
#[utoipa::path(
    get,
    path = "",
    params(NoteQueryParams),
    responses(
        (status = 200, description = "Successfully got notes", body = [Note]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No notes found"),
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
    let ids = search_notes(&state, params, &mut conn).await?;
    let notes = schema::notes::table
        .select(Note::as_select())
        .filter(schema::notes::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(notes))
}
