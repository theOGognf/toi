use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{models::search::SimilaritySearchParams, utils};

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
    /// Parameters for performing similarity search against notes.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
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
