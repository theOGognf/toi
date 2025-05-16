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
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to specific words or phrases, whereas `false` is useful for more broad
    /// matching.
    #[serde(default)]
    pub use_reranking_filter: bool,
}
