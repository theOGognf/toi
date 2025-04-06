use chrono::{DateTime, Utc};
use diesel::{Queryable, Selectable, prelude::Insertable};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::utils;

#[derive(Queryable, Selectable, Serialize, ToSchema)]
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

#[derive(Deserialize, ToSchema)]
pub struct NewNoteRequest {
    /// Note content to add.
    pub content: String,
}

#[derive(Deserialize, IntoParams)]
pub struct NoteQueryParams {
    /// Parameters for performing similarity search against notes.
    pub similarity_search_params: Option<NoteSimilaritySearchParams>,
    /// Filter on notes created after this ISO formatted datetime.
    pub from: Option<DateTime<Utc>>,
    /// Filter on notes created before this ISO formatted datetime.
    pub to: Option<DateTime<Utc>>,
    /// How to order results for retrieved notes.
    pub order_by: Option<utils::OrderBy>,
    /// Max number of notes to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct NoteSimilaritySearchParams {
    /// Query string to compare notes to.
    pub query: String,
    /// Measure of difference between the query and notes it's being
    /// compared to. Only return notes whose distance is less than
    /// or equal this value.
    #[serde(default = "utils::default_distance_threshold")]
    #[schema(minimum = 0.1, maximum = 0.5)]
    pub distance_threshold: f64,
}
