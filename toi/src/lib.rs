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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GenerationRequest {
    pub messages: Vec<Message>,
    #[serde(skip_deserializing)]
    response_format: Option<Value>,
}

impl GenerationRequest {
    #[must_use]
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            response_format: None,
        }
    }

    #[must_use]
    pub fn with_response_format(mut self, value: Value) -> Self {
        self.response_format = Some(value);
        self
    }
}
