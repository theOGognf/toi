use axum::{extract::State, http::StatusCode, response::Json};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        notes::{NewNote, NewNoteRequest, Note, NoteSearchParams},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find notes similar to the one the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn notes_router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(add_note,))
        .routes(routes!(delete_matching_notes))
        .routes(routes!(get_matching_notes))
        .with_state(state)
}

async fn search_notes(
    state: &ToiState,
    params: NoteSearchParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let NoteSearchParams {
        ids,
        query,
        use_reranking_filter,
        created_from,
        created_to,
        order_by,
        limit,
    } = params;

    let mut sql_query = schema::notes::table.select(Note::as_select()).into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = created_from {
        sql_query = sql_query.filter(schema::notes::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = created_to {
        sql_query = sql_query.filter(schema::notes::created_at.le(created_to));
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => sql_query = sql_query.order(schema::notes::created_at),
        Some(utils::OrderBy::Newest) => {
            sql_query = sql_query.order(schema::notes::created_at.desc());
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
        sql_query = sql_query.or_filter(schema::notes::id.eq_any(ids));
    }

    // Limit number of items.
    if let Some(limit) = limit {
        sql_query = sql_query.limit(limit);
    }

    // Get all the items that match the query.
    let notes: Vec<Note> = sql_query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) = notes
        .into_iter()
        .map(|note| (note.id, note.content))
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
    extensions(
        ("x-json-schema-body" = json!(schema_for!(NewNoteRequest)))
    ),
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
    Json(params): Json<NewNoteRequest>,
) -> Result<Json<Note>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let NewNoteRequest { content } = params;
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
    post,
    path = "/delete",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(NoteSearchParams)))
    ),
    request_body = NoteSearchParams,
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
    Json(params): Json<NoteSearchParams>,
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
    post,
    path = "/search",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(NoteSearchParams)))
    ),
    request_body = NoteSearchParams,
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
    Json(params): Json<NoteSearchParams>,
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
