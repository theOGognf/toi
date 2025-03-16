use chrono::{DateTime, Utc};
use diesel::{Queryable, Selectable, prelude::Insertable};
use pgvector::Vector;
use utoipa::ToSchema;

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
