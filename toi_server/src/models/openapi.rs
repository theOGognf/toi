use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use serde::Serialize;
use serde_json::Value;

#[derive(Insertable, Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::openapi)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OpenApiPathItem {
    pub path: String,
    pub method: String,
    pub description: String,
    pub params: Option<Value>,
    pub body: Option<Value>,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::searchable_openapi)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchableOpenApiPathItem {
    pub parent_id: i32,
    pub description: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::searchable_openapi)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewSearchableOpenApiPathItem {
    pub parent_id: i32,
    pub description: String,
    pub embedding: Vector,
}
