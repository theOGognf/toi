use serde::{Deserialize, Serialize};
use serde_query::DeserializeQuery;

#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    #[serde(skip)]
    System,
    Assistant,
    User,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Deserialize, Serialize)]
pub struct GenerationRequest {
    pub messages: Vec<Message>,
}

#[derive(Deserialize, Serialize, DeserializeQuery)]
pub struct GenerationResponse {
    #[query(".choices.[0].message.content")]
    pub content: String,
}
