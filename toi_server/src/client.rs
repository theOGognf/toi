use axum::{body::Body, http::StatusCode};
use pgvector::Vector;
use reqwest::{Client, header::HeaderMap};
use serde::{Serialize, de::DeserializeOwned};
use toi::GenerationRequest;

use crate::models::client::{
    EmbeddingRequest, EmbeddingResponse, GenerationResponse, HttpClientConfig, ModelClientError,
    StreamingGenerationRequest,
};

#[derive(Clone)]
pub struct ModelClient {
    pub embedding_api_config: HttpClientConfig,
    embedding_client: Client,
    pub generation_api_config: HttpClientConfig,
    generation_client: Client,
}

impl ModelClient {
    fn build_request_json<Request: Serialize>(
        config: &HttpClientConfig,
        request: Request,
    ) -> Result<serde_json::Map<String, serde_json::Value>, (StatusCode, String)> {
        let mut value = serde_json::to_value(request)
            .map_err(|err| ModelClientError::RequestJson.into_response(&err.to_string()))?;
        let request = value
            .as_object_mut()
            .expect("request value can never be empty");
        if let Some(json) = serde_json::to_value(config.json.clone())
            .map_err(|err| ModelClientError::DefaultJson.into_response(&err.to_string()))?
            .as_object()
        {
            request.extend(json.clone());
        }
        Ok(request.clone())
    }

    pub async fn embed(&self, request: EmbeddingRequest) -> Result<Vector, (StatusCode, String)> {
        let response: EmbeddingResponse = Self::post(
            &self.embedding_api_config,
            "/v1/embeddings".to_string(),
            &self.embedding_client,
            request,
        )
        .await?;
        Ok(Vector::from(response.embedding))
    }

    pub async fn generate(
        &self,
        request: GenerationRequest,
    ) -> Result<String, (StatusCode, String)> {
        let response: GenerationResponse = Self::post(
            &self.generation_api_config,
            "/v1/chat/completions".to_string(),
            &self.generation_client,
            request,
        )
        .await?;
        Ok(response.content)
    }

    pub async fn generate_stream(
        &self,
        request: StreamingGenerationRequest,
    ) -> Result<Body, (StatusCode, String)> {
        let base_url = self.generation_api_config.base_url.trim_end_matches("/");
        let url = format!("{base_url}/v1/chat/completions");
        let request = Self::build_request_json(&self.generation_api_config, request)?;
        let response = self
            .generation_client
            .post(&url)
            .query(&self.generation_api_config.params)
            .json(&request)
            .send()
            .await
            .map_err(|err| ModelClientError::ApiConnection.into_response(&format!("{err:?}")))?;
        let stream = response.bytes_stream();
        Ok(Body::from_stream(stream))
    }

    pub fn new(
        embedding_api_config: HttpClientConfig,
        generation_api_config: HttpClientConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let embedding_header_map = HeaderMap::try_from(&embedding_api_config.headers)?;
        let embedding_client = Client::builder()
            .default_headers(embedding_header_map)
            .build()?;
        let generation_header_map = HeaderMap::try_from(&generation_api_config.headers)?;
        let generation_client = Client::builder()
            .default_headers(generation_header_map)
            .build()?;
        Ok(Self {
            embedding_api_config,
            embedding_client,
            generation_api_config,
            generation_client,
        })
    }

    async fn post<Request: Serialize, ResponseModel: DeserializeOwned>(
        config: &HttpClientConfig,
        endpoint: String,
        client: &Client,
        request: Request,
    ) -> Result<ResponseModel, (StatusCode, String)> {
        let base_url = config.base_url.trim_end_matches("/");
        let url = format!("{base_url}{endpoint}",);
        let request = Self::build_request_json(config, request)?;
        client
            .post(&url)
            .query(&config.params)
            .json(&request)
            .send()
            .await
            .map_err(|err| ModelClientError::ApiConnection.into_response(&format!("{err:?}")))
            .json::<ResponseModel>()
            .await
            .map_err(|err| ModelClientError::ResponseJson.into_response(&format!("{err:?}")))
    }
}
