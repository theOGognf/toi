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
    #[schema(value_type = Vec<f32>)]
    pub embedding: Vector,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, serde::Deserialize, ToSchema)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewNote {
    pub content: String,
    #[schema(value_type = Vec<f32>)]
    pub embedding: Vector,
}
