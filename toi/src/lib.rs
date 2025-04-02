pub mod models;
use std::error::Error;

pub use models::client::{GenerationRequest, GenerationResponse, Message, MessageRole};

pub fn detailed_reqwest_error(err: reqwest::Error) -> String {
    let mut repr = err.to_string();
    if let Some(source) = err.source() {
        repr = format!("{repr} from {source}");
    }
    if let Some(url) = err.url() {
        repr = format!("{repr} at {url}");
    }
    repr
}
