use bon::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils;

#[derive(Builder, Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct SimilaritySearchParams {
    /// Query string to compare to.
    pub query: String,
    /// Measure of difference between the query and string it's being
    /// compared to. Only return items whose distance is less than
    /// or equal this value.
    #[serde(default = "utils::default_distance_threshold")]
    #[schema(minimum = 0.1, maximum = 0.5)]
    pub distance_threshold: f64,
}
