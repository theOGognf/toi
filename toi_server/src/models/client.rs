use std::collections::HashMap;

#[derive(serde::Serialize)]
pub struct EmbeddingRequest {
    pub input: String,
}

#[derive(serde::Deserialize, serde_query::DeserializeQuery)]
pub struct EmbeddingResponse {
    #[query(".data.[0].embedding")]
    pub embedding: Vec<f32>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    Assistant,
    User,
}

#[derive(serde::Serialize)]
pub struct Message {
    role: MessageRole,
    content: String,
}

#[derive(serde::Serialize)]
pub struct GenerateRequest {
    pub messages: Vec<Message>,
}

#[derive(serde::Deserialize, serde_query::DeserializeQuery)]
pub struct GenerateResponse {
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
