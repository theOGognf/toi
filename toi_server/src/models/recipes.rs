use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{models::tags::Tag, utils};

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::recipes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Recipe {
    /// Unique recipe ID.
    pub id: i32,
    /// Recipe title or description.
    pub description: String,
    /// Recipe ingredients.
    pub ingredients: String,
    /// Recipe instructions.
    pub instructions: String,
    /// Datetime the recipe was created in ISO format.
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::recipes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RecipePreview {
    /// Unique recipe ID.
    pub id: i32,
    /// Recipe title or description.
    pub description: String,
    /// Datetime the recipe was created in ISO format.
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct RecipeTags {
    /// Matching recipe preview.
    pub recipe_preview: RecipePreview,
    /// Matching tags.
    pub tags: Vec<Tag>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::recipes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRecipe {
    pub description: String,
    pub ingredients: String,
    pub instructions: String,
    pub embedding: Vector,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::recipe_tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRecipeTag {
    pub recipe_id: i32,
    pub tag_id: i32,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct NewRecipeRequest {
    /// Recipe title or description.
    pub description: String,
    /// Recipe ingredients.
    pub ingredients: String,
    /// Recipe instructions.
    pub instructions: String,
    /// Recipe tags.
    pub tags: Vec<String>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct RecipeSearchParams {
    /// Update a recipe using their database-generated ID rather than
    /// searching for them.
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
    /// Filter on recipes created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on recipes created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved recipes.
    pub order_by: Option<utils::OrderBy>,
    /// Recipe tags to search with.
    pub tags: Option<Vec<String>>,
    /// Limit the max number of recipes to return from the search.
    pub limit: Option<i64>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct RecipeTagSearchParams {
    /// Select an recipe using its database-generated IDs rather than
    /// searching for it first.
    pub recipe_id: Option<i32>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub recipe_query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to a specific phrase, name, or words.
    pub recipe_use_reranking_filter: Option<bool>,
    /// Filter on recipes created after this ISO formatted datetime.
    pub recipe_created_from: Option<DateTime<Utc>>,
    /// Filter on recipes created before this ISO formatted datetime.
    pub recipe_created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved recipes.
    pub recipe_order_by: Option<utils::OrderBy>,
    /// Search tags using their database-generated IDs rather than
    /// searching for them first.
    pub tag_ids: Option<Vec<i32>>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub tag_query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to a specific phrase, name, or words.
    pub tag_use_reranking_filter: Option<bool>,
    /// Whether to match the query string more closely, character-for-character.
    pub tag_use_edit_distance_filter: Option<bool>,
    /// Limit the max number of tags to return from the search.
    pub tag_limit: Option<i64>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct NewRecipeTagsRequest {
    /// Update a recipe using their database-generated ID rather than
    /// searching for them.
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
    /// Filter on recipes created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on recipes created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved recipes.
    pub order_by: Option<utils::OrderBy>,
    /// Recipe tags to add.
    pub tags: Vec<String>,
    /// Limit the max number of recipes to return from the search.
    pub limit: Option<i64>,
}
