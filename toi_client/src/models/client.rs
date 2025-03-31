use serde::Deserialize;
use serde_query::DeserializeQuery;

#[derive(Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

#[derive(Deserialize, DeserializeQuery)]
pub struct GenerationResponseChunk {
    #[query(".choices.[0].delta.content")]
    pub content: String,

    #[query(".usage")]
    pub usage: Option<TokenUsage>,
}
