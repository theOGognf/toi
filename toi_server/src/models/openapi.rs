use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// We only care about retrieving the actual spec for request generation.
/// The ID and actual embeddings are irrelevant.
#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::openapi)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OpenApiPathItem {
    pub path: String,
    pub method: String,
    pub params: Option<Value>,
    pub body: Option<Value>,
}

#[derive(AsChangeset, Insertable)]
#[diesel(table_name = crate::schema::openapi)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewOpenApiPathItem {
    pub path: String,
    pub method: String,
    pub params: Option<Value>,
    pub body: Option<Value>,
    pub embedding: Vector,
}
