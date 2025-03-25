use std::collections::HashMap;

pub const MAX_CHAT_HISTORY_SIZE: usize = 10;

#[derive(serde::Serialize)]
pub struct EmbeddingRequest {
    pub input: String,
}

#[derive(serde::Deserialize, serde_query::DeserializeQuery)]
pub struct EmbeddingResponse {
    #[query(".data.[0].embedding")]
    pub embedding: Vec<f32>,
}

#[derive(Clone, serde::Deserialize, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    #[serde(skip)]
    System,
    Assistant,
    User,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct GenerationRequest {
    pub messages: Vec<Message>,
}

#[derive(serde::Deserialize, serde::Serialize, serde_query::DeserializeQuery)]
pub struct GenerationResponse {
    #[query(".choices.[0].message.content")]
    pub content: String,
}

#[derive(Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct HttpClientConfig {
    pub base_url: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub json: HashMap<String, String>,
}
