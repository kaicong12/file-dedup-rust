use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub jwt_secret: String,
    pub database_url: String,
    pub aws_profile_name: String,
    pub s3_bucket_name: String,
    pub s3_document_prefix: String,
    pub redis_url: String,
    pub opensearch_url: String,
    pub bedrock_model_id: String,
    pub otel_exporter_otlp_endpoint: String,
}

impl Config {
    pub fn initialize(env_path: &str) -> Self {
        dotenv::from_path(env_path).ok();
        envy::from_env::<Config>().expect("Failed to load configuration from env")
    }
}
