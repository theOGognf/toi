use std::collections::HashMap;

#[derive(serde::Deserialize, serde_query::DeserializeQuery)]
pub struct EmbeddingResponse {
    #[query(".data.[0].embedding")]
    pub embedding: Vec<f32>,
}

pub struct GenerateRequest {}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub struct HttpClientConfig {
    pub base_url: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub json: HashMap<String, String>,
}
