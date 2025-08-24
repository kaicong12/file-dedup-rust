use aws_sdk_bedrockruntime::{Client, primitives::Blob};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_encode;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};

pub struct Deduplicator;

impl Deduplicator {
    async fn get_bedrock_client(profile_name: &str) -> Client {
        let config = aws_config::from_env()
            .profile_name(profile_name)
            .load()
            .await;

        Client::new(&config)
    }

    pub async fn generate_embeddings(
        profile_name: &str,
        input: &str,
        model_id: &str,
    ) -> Result<Vec<f64>, String> {
        let client = Self::get_bedrock_client(profile_name).await;
        let response_template = json!({
            "inputText": input
        });

        let vectorized_input =
            serde_json::to_vec(&response_template).map_err(|err| err.to_string())?;

        let response = client
            .invoke_model()
            .model_id(model_id)
            .content_type("application/json")
            .body(Blob::new(vectorized_input))
            .send()
            .await
            .map_err(|err| err.to_string())?;

        let response_bytes = response.body().clone().into_inner();
        let resp: serde_json::Value =
            serde_json::from_slice(&response_bytes).map_err(|err| err.to_string())?;

        // as_f64.ok_or() produces a Result<f64, _), mapping over it produces Vec<Result<f64, _>>
        // however the collect method collects this iterator into <Result<Vec<f64>, _>>
        // IF AND ONLY IF every element is Ok, if any throws an Err, it will return the Err
        let result = resp["embedding"]
            .as_array()
            .ok_or("Failed to extract embeddings array")?
            .iter()
            .map(|v| v.as_f64().ok_or("Invalid embedding value"))
            .collect::<Result<Vec<f64>, _>>()
            .map_err(|err| err.to_string())?;

        Ok(result)
    }

    pub fn generate_sha256_for_file(file_path: &str) -> Result<String, io::Error> {
        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 4096]; // 4 KB buffer

        loop {
            // read a chunk of 4KB each iteration
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // End of file
            }
            hasher.update(&buffer[..bytes_read]); // Update hash with the chunk
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn generate_base64_for_image(file_path: &str) -> Result<String, io::Error> {
        // base64 encoding for an image file
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(base64_encode.encode(&buffer))
    }
}

#[cfg(test)]
mod deduplicator_test {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_generate_embeddings() {
        let model_id = "amazon.titan-embed-image-v1";
        let input_text = "Sample text to generate embeddings";

        let embeddings =
            Deduplicator::generate_embeddings("sso_profile", input_text, model_id).await;

        assert!(embeddings.is_ok());
    }

    #[test]
    fn test_generate_base64_for_image() {
        let file_path = "src/worker/test_data/spiderman_meme.jpg"; // Replace with a valid image path
        let base64_result = Deduplicator::generate_base64_for_image(file_path);

        assert!(base64_result.is_ok());
    }

    #[test]
    fn test_generate_sha256_for_file() {
        let file_path = "src/worker/test_data/sample_text.txt"; // Replace with a valid file path
        let sha256_result = Deduplicator::generate_sha256_for_file(file_path);

        assert!(sha256_result.is_ok());
    }
}
