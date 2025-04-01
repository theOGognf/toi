use serde::{Deserialize, Serialize};
use serde_query::DeserializeQuery;
use utoipa::ToSchema;

#[derive(Clone, Deserialize, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    #[serde(skip_deserializing)]
    System,
    Assistant,
    User,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct GenerationRequest {
    pub messages: Vec<Message>,
}

#[derive(Deserialize, Serialize, DeserializeQuery)]
pub struct GenerationResponse {
    #[query(".choices.[0].message.content")]
    pub content: String,
}
