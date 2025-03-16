use diesel::{Queryable, Selectable};
use pgvector::Vector;
use chrono::{DateTime, Utc};

#[derive(Queryable, Selectable, serde::Serialize)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Note {
    pub id: i32,
    pub content: String,
    pub embedding: Vector,
    pub created_at: DateTime<Utc>,
}
