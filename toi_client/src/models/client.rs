use serde::Deserialize;

#[derive(Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

#[derive(Deserialize)]
pub struct Delta {
    pub content: String,
}

#[derive(Deserialize)]
pub struct Choice {
    pub delta: Delta,
}

#[derive(Deserialize)]
pub struct GenerationResponseChunk {
    pub choices: Vec<Choice>,
    pub usage: Option<TokenUsage>,
}
