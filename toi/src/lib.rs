use bon::Builder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    #[serde(skip_deserializing)]
    System,
    Assistant,
    User,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Builder, Debug, Deserialize, Serialize, ToSchema)]
pub struct GenerationRequest {
    pub messages: Vec<Message>,
    #[serde(skip_deserializing)]
    response_format: Option<Value>,
}
