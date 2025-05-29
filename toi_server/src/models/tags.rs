use bon::Builder;
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct NewTagRequest {
    /// Tag name to add.
    pub name: String,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct TagSearchParams {
    /// Select tags using their database-generated IDs rather than searching
    /// for them.
    pub ids: Option<Vec<i32>>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question. This can be left empty to ignore
    /// similarity search in cases where the user wants to filter by
    /// other means or get all items.
    pub query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to specific words or phrases, whereas `false` is useful for more broad
    /// matching.
    pub use_reranking_filter: Option<bool>,
    /// Whether to match the query string more closely, character-for-character.
    pub use_edit_distance_filter: Option<bool>,
    /// Limit the max number of tags to return from the search.
    pub limit: Option<i64>,
}
