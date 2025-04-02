use serde::Deserialize;

#[derive(Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

#[derive(serde_query::Deserialize)]
pub struct GenerationResponseChunk {
    #[query(".choices.[].delta.content")]
    pub content: Vec<String>,

    #[query(".usage")]
    pub usage: Option<TokenUsage>,
}
