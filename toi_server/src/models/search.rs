use bon::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Builder, Clone, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct SimilaritySearchParams {
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question. This can be left empty to ignore
    /// similarity search in cases where the user wants to filter by
    /// other means or get all items.
    pub query: String,
    /// Measure of distance between the query and string it's being
    /// compared to. Only return items whose distance is less than
    /// or equal this value. A lower number restricts the search to
    /// more similar items, while a higher number allows for more
    /// dissimilar items. This defaults to the server-configured
    /// value if left null, which is usually fine for most scenarios.
    #[schema(minimum = 0.01, maximum = 0.95)]
    pub distance_threshold: Option<f64>,
    /// Measure of similarity between the query and string it's being
    /// compared to. Only return items whose distance is greater than
    /// or equal this value. A higher number restricts the search to
    /// more similar items, while a lower number allows for more
    /// dissimilar items. This defaults to the server-configured
    /// value if left null, which is usually fine for most scenarios.
    #[schema(minimum = 0.01, maximum = 0.95)]
    pub similarity_threshold: Option<f64>,
}
