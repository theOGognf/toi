use toi::Message;
use crate::models::client::GenerationResponseChunk;

pub enum UserRequest {
    Prompt(String),
    Cancel,
}

pub enum ServerRequest {
    Start(Vec<Message>),
    Cancel,
}

pub enum ServerResponse {
    Chunk(GenerationResponseChunk),
    Done,
    Error(String),
}
