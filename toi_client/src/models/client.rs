use serde::Deserialize;

#[derive(Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

#[derive(Deserialize)]
pub struct StreamingDelta {
    pub content: String,
}

#[derive(Deserialize)]
pub struct StreamingChoice {
    pub delta: StreamingDelta,
}

#[derive(Deserialize)]
pub struct GenerationResponseChunk {
    pub choices: Vec<StreamingChoice>,
    pub usage: Option<TokenUsage>,
}
