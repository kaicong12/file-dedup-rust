use reqwest::{Client, header::HeaderValue};
use serde::{self, Deserialize, Serialize};
use std::time::Duration;

pub struct OpenAIClient {
    client: Client,
    api_key: String,
    base_url: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: EmbeddingInput,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: EmbeddingObject,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingObject {
    pub embedding: Vec<f64>,
    index: usize,
}

impl OpenAIClient {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .pool_max_idle_per_host(8)
            .build()
            .expect("reqwest client");

        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }

    pub async fn generate_embeddings(
        &self,
        input_text: EmbeddingInput,
    ) -> Result<EmbeddingObject, String> {
        let embedding_model = "text-embedding-3-small";
        let embeddings_url = format!(
            "{}/{}/embeddings",
            self.base_url.trim_end_matches("/"),
            embedding_model
        );
        println!("Embedding URL: {embeddings_url}");
        let request_body = EmbeddingRequest {
            input: input_text.into(),
        };

        let resp = self
            .client
            .post(embeddings_url)
            .bearer_auth(&self.api_key)
            .query(&[("api-version", "2024-10-21")])
            .header("X-Merck-APIKey", &self.api_key)
            .json(&request_body)
            .send()
            .await
            .map_err(|err| err.to_string())?;

        let raw_response = resp.json().await.map_err(|err| err.to_string())?;
        println!("Raw response: {raw_response:?}");
        let embedding_response: EmbeddingResponse =
            serde_json::from_value(raw_response).map_err(|err| err.to_string())?;

        Ok(embedding_response.data)
    }
}
