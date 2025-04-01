use crate::models::client::GenerationResponseChunk;
use toi::GenerationRequest;

pub enum UserRequest {
    Prompt(String),
    Cancel,
}

pub enum ServerRequest {
    Start(GenerationRequest),
    Cancel,
}

pub enum ServerResponse {
    Chunk(GenerationResponseChunk),
    Done,
    Error(String),
}
