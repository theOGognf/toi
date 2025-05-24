use bon::Builder;
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::models::search::SimilaritySearchParams;

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Tag {
    /// Unique tag ID.
    pub id: i32,
    /// Tag name.
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTag {
    pub name: String,
    pub embedding: Vector,
}

#[derive(Deserialize, JsonSchema, ToSchema)]
pub struct NewTagRequest {
    /// Tag name to add.
    pub name: String,
}

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize)]
pub struct TagQueryParams {
    /// Select tags using their database-generated IDs rather than searching
    /// for them.
    pub ids: Option<Vec<i32>>,
    /// Parameters for performing similarity search against tags.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Limit the max number of tags to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}
