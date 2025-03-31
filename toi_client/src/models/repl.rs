use crate::models::client::GenerationResponseChunk;
use toi::Message;

pub enum UserRequest {
    Prompt(String),
    Cancel,
    Quit,
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
