use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Queryable, Selectable, prelude::Insertable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{models::search::SimilaritySearchParams, utils};

#[derive(Clone, Deserialize, Queryable, Selectable, Serialize, ToSchema)]
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

#[derive(Builder, Deserialize, Serialize, JsonSchema, IntoParams)]
pub struct NoteQueryParams {
    /// Parameters for performing similarity search against notes.
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Filter on notes created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on notes created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved notes.
    pub order_by: Option<utils::OrderBy>,
    /// Max number of notes to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}
