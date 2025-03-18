use crate::models;

pub struct Client {
    pub embedding_api_config: models::client::HttpClientConfig,
    pub embedding_client: reqwest::Client,
    pub generation_api_config: models::client::HttpClientConfig,
    pub generation_client: reqwest::Client,
}

impl Client {
    pub async fn embed(self, content: String) -> reqwest::Result<Vec<f32>> {
        let base_url = self.embedding_api_config.base_url.trim_end_matches("/");
        let url = format!("{base_url}/embeddings",);
        let mut json = self.embedding_api_config.json.clone();
        json.insert("input".to_string(), content);
        let resp = self
            .embedding_client
            .post(url)
            .query(&self.embedding_api_config.params)
            .json(&json)
            .send()
            .await?
            .json::<models::client::EmbeddingResponse>()
            .await?;
        Ok(resp.embedding)
    }

    pub async fn generate() -> reqwest::Result<String> {}

    pub fn new(
        embedding_api_config: models::client::HttpClientConfig,
        generation_api_config: models::client::HttpClientConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let embedding_header_map =
            reqwest::header::HeaderMap::try_from(&embedding_api_config.headers)?;
        let embedding_client = reqwest::Client::builder()
            .default_headers(embedding_header_map)
            .build()?;
        let generation_header_map =
            reqwest::header::HeaderMap::try_from(&generation_api_config.headers)?;
        let generation_client = reqwest::Client::builder()
            .default_headers(generation_header_map)
            .build()?;
        Ok(Self {
            embedding_api_config,
            embedding_client,
            generation_api_config,
            generation_client,
        })
    }
}
