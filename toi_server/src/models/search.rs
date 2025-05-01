use bon::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils;

#[derive(Builder, Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct SimilaritySearchParams {
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    pub query: String,
    /// Measure of distance between the query and string it's being
    /// compared to. Only return items whose distance is less than
    /// or equal this value. A lower number restricts the search to
    /// more similar items, while a higher number allows for more
    /// dissimilar items. The default value for this is usually fine
    /// for most scenarios.
    #[serde(default = "utils::default_distance_threshold")]
    #[schema(minimum = 0.01, maximum = 0.95)]
    pub distance_threshold: f64,
    /// Measure of similarity between the query and string it's being
    /// compared to. Only return items whose distance is greater than
    /// or equal this value. A higher number restricts the search to
    /// more similar items, while a lower number allows for more
    /// dissimilar items. The default value for this is usually fine
    /// for most scenarios.
    #[serde(default = "utils::default_similarity_threshold")]
    #[schema(minimum = 0.01, maximum = 0.95)]
    pub similarity_threshold: f64,
}
