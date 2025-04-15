use diesel::{Queryable, Selectable, prelude::Insertable};
use pgvector::Vector;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::openapi)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OpenApiPath {
    /// Unique OpenAPI path ID.
    pub id: i32,
    /// OpenAPI spec content.
    pub spec: Value,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::openapi)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewOpenApiPath {
    pub spec: Value,
    pub embedding: Vector,
}
