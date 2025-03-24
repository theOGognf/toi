use axum::{body::Body, http::StatusCode};
use pgvector::Vector;

use crate::models;

#[derive(Clone)]
pub struct Client {
    embedding_api_config: models::client::HttpClientConfig,
    embedding_client: reqwest::Client,
    generation_api_config: models::client::HttpClientConfig,
    generation_client: reqwest::Client,
}

impl Client {
    fn build_request_json<Request: serde::ser::Serialize>(
        config: &models::client::HttpClientConfig,
        request: Request,
    ) -> Result<serde_json::Map<String, serde_json::Value>, (StatusCode, String)> {
        let mut value = serde_json::to_value(request)
            .map_err(|err| (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()))?;
        let request = value
            .as_object_mut()
            .expect("Request value can never be empty");
        if let Some(json) = serde_json::to_value(config.json.clone())
            .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
            .as_object()
        {
            request.extend(json.clone());
        }
        Ok(request.clone())
    }

    pub async fn embed(
        self,
        request: models::client::EmbeddingRequest,
    ) -> Result<Vector, (StatusCode, String)> {
        let resp: models::client::EmbeddingResponse = Self::post(
            self.embedding_api_config,
            "/embeddings".to_string(),
            self.embedding_client,
            request,
        )
        .await?;
        Ok(Vector::from(resp.embedding))
    }

    pub async fn generate(
        self,
        request: models::client::GenerateRequest,
    ) -> Result<String, (StatusCode, String)> {
        let resp: models::client::GenerateResponse = Self::post(
            self.generation_api_config,
            "/chat/completions".to_string(),
            self.generation_client,
            request,
        )
        .await?;
        Ok(resp.content)
    }

    pub async fn generate_stream(
        self,
        request: models::client::GenerateRequest,
    ) -> Result<Body, (StatusCode, String)> {
        let base_url = self.generation_api_config.base_url.trim_end_matches("/");
        let url = format!("{base_url}{}", "/chat/completions");
        let request = Self::build_request_json(&self.generation_api_config, request)?;
        let response = self
            .generation_client
            .post(url)
            .query(&self.generation_api_config.params)
            .json(&request)
            .send()
            .await
            .map_err(|err| (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()))?;
        let stream = response.bytes_stream();
        Ok(Body::from_stream(stream))
    }

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

    async fn post<Request: serde::ser::Serialize, ResponseModel: serde::de::DeserializeOwned>(
        config: models::client::HttpClientConfig,
        endpoint: String,
        client: reqwest::Client,
        request: Request,
    ) -> Result<ResponseModel, (StatusCode, String)> {
        let base_url = config.base_url.trim_end_matches("/");
        let url = format!("{base_url}{endpoint}",);
        let request = Self::build_request_json(&config, request)?;
        client
            .post(url)
            .query(&config.params)
            .json(&request)
            .send()
            .await
            .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?
            .json::<ResponseModel>()
            .await
            .map_err(|err| (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()))
    }
}
