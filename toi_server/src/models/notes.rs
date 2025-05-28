use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::utils;

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Note {
    /// Unique note ID.
    pub id: i32,
    /// Note content.
    pub content: String,
    /// Datetime the note was created in ISO format.
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewNote {
    pub content: String,
    pub embedding: Vector,
}

#[derive(Deserialize, JsonSchema, ToSchema)]
pub struct NewNoteRequest {
    /// Note content to add.
    pub content: String,
}

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize)]
pub struct NoteQueryParams {
    /// Select notes using their database-generated IDs rather than searching
    /// for them.
    pub ids: Option<Vec<i32>>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question. This can be left empty to ignore
    /// similarity search in cases where the user wants to filter by
    /// other means or get all items.
    pub query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to specific words or phrases, whereas `false` is useful for more broad
    /// matching.
    pub use_reranking_filter: Option<bool>,
    /// Filter on notes created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on notes created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved notes.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of notes to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}
