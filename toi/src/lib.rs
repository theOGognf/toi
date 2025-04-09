use serde::{Deserialize, Serialize};
use std::error::Error;
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

pub fn detailed_reqwest_error(err: reqwest::Error) -> String {
    let mut repr = err.to_string();
    if let Some(source) = err.source() {
        repr = format!("{repr} from {source}");
    }
    repr
}
