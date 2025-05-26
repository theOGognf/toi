use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{models::search::SimilaritySearchParams, utils};

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

#[derive(Deserialize, JsonSchema, ToSchema)]
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

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize, ToSchema)]
pub struct RecipeQueryParams {
    /// Update a recipe using their database-generated ID rather than
    /// searching for them.
    pub ids: Option<Vec<i32>>,
    /// Parameters for performing similarity search against recipes.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Filter on recipes created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on recipes created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved events.
    pub order_by: Option<utils::OrderBy>,
    /// Recipe tags to search with.
    pub tags: Option<Vec<String>>,
    /// Limit the max number of recipes to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize, ToSchema)]
pub struct NewRecipeTagsRequest {
    /// Update a recipe using their database-generated ID rather than
    /// searching for them.
    pub ids: Option<Vec<i32>>,
    /// Parameters for performing similarity search against recipes.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Filter on recipes created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on recipes created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved events.
    pub order_by: Option<utils::OrderBy>,
    /// Recipe tags to add.
    pub tags: Vec<String>,
    /// Limit the max number of recipes to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}
