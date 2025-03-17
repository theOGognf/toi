use chrono::{DateTime, Utc};
use diesel::{Queryable, Selectable, prelude::Insertable};
use pgvector::Vector;
use utoipa::{IntoParams, ToSchema};

#[derive(Queryable, Selectable, serde::Serialize, ToSchema)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Note {
    pub id: i32,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewNote {
    pub content: String,
    pub embedding: Vector,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct NewNoteRequest {
    pub content: String,
}

#[derive(serde::Deserialize, IntoParams)]
pub struct NoteQueryParams {
    pub similarity_search_params: Option<NoteSimilaritySearchParams>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct NoteSimilaritySearchParams {
    pub query: String,
    pub threshold: f64,
    pub limit: i64,
}
