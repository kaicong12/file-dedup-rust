use crate::worker::deduplicator::Deduplicator;
use crate::worker::job_queue::{DeduplicationJob, JobQueue};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use std::rc::Rc;
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
pub struct SimilarFile {
    pub file_id: i32,
    pub file_name: String,
    pub sha256_hash: String,
    pub similarity_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeduplicationResult {
    pub file_id: i32,
    pub sha256_hash: String,
    pub exact_duplicates: Vec<i32>,
    pub similar_files: Vec<SimilarFile>,
    pub cluster_id: Option<i32>,
}

pub struct DeduplicationService {
    db_pool: PgPool,
    job_queue: JobQueue,
    opensearch_client: Client,
    opensearch_url: String,
    aws_profile: String,
    bedrock_model_id: String,
}

impl DeduplicationService {
    pub fn new(
        db_pool: PgPool,
        job_queue: JobQueue,
        opensearch_url: String,
        aws_profile: String,
        bedrock_model_id: String,
    ) -> Self {
        let opensearch_client = Client::new();

        Self {
            db_pool,
            job_queue,
            opensearch_client,
            opensearch_url,
            aws_profile,
            bedrock_model_id,
        }
    }

    pub fn with_metrics(
        db_pool: PgPool,
        job_queue: JobQueue,
        opensearch_url: String,
        aws_profile: String,
        bedrock_model_id: String,
    ) -> Self {
        let opensearch_client = Client::new();

        Self {
            db_pool,
            job_queue,
            opensearch_client,
            opensearch_url,
            aws_profile,
            bedrock_model_id,
        }
    }

    fn get_opensearch_index(&self, file_name: &str) -> String {
        if self.is_image_file(file_name) {
            "image-embeddings".to_string()
        } else {
            "file-embeddings".to_string()
        }
    }

    pub async fn process_deduplication_job(&self, job: DeduplicationJob) -> Result<()> {
        log::info!("Processing deduplication job: {}", job.job_id);

        let start_time = Instant::now();

        match self.perform_deduplication(&job).await {
            Ok(result) => {
                let duration = start_time.elapsed();

                log::info!(
                    "Deduplication completed for file_id: {}, found {} exact duplicates, {} similar files in {:.2}s",
                    job.file_id,
                    result.exact_duplicates.len(),
                    result.similar_files.len(),
                    duration.as_secs_f64()
                );

                self.job_queue
                    .update_job_status(&job.job_id, "completed", None)
                    .await?;
            }
            Err(e) => {
                log::error!("Deduplication failed for job {}: {}", job.job_id, e);
                self.job_queue
                    .update_job_status(&job.job_id, "failed", Some(e.to_string()))
                    .await?;
                return Err(e);
            }
        }

        Ok(())
    }

    async fn perform_deduplication(&self, job: &DeduplicationJob) -> Result<DeduplicationResult> {
        // Step 1: Get file info and generate SHA256 hash
        let file_info = self.get_file_info(job.file_id).await?;
        let sha256_hash = self.generate_file_hash(&job.s3_key).await?;

        // Step 2: Check for exact duplicates using SHA256
        let exact_duplicates = self
            .find_exact_duplicates(&sha256_hash, job.file_id)
            .await?;

        // Step 3: Generate embeddings for the file
        let embeddings = self
            .generate_file_embeddings(&job.s3_key, &job.file_name)
            .await?;

        // Step 4: Store embeddings in OpenSearch
        self.store_embeddings_in_opensearch(job.file_id, &job.file_name, &sha256_hash, &embeddings)
            .await?;

        // Step 5: Find similar files using embeddings
        let similar_files = self
            .find_similar_files(&embeddings, job.file_id, &job.file_name)
            .await?;

        // Step 6: Update database with results
        let cluster_id = self
            .update_file_clusters(job.file_id, &exact_duplicates, &similar_files)
            .await?;

        // Step 7: Update file record with SHA256 hash
        self.update_file_hash(job.file_id, &sha256_hash).await?;

        Ok(DeduplicationResult {
            file_id: job.file_id,
            sha256_hash,
            exact_duplicates,
            similar_files,
            cluster_id,
        })
    }

    async fn get_file_info(&self, file_id: i32) -> Result<(String, i64)> {
        let row = sqlx::query("SELECT file_name FROM File WHERE file_id = $1")
            .bind(file_id)
            .fetch_one(&self.db_pool)
            .await?;

        let file_name: String = row.get("file_name");
        // For now, we'll set file_size to 0 as it's not in the current schema
        // You might want to add this to your File table
        Ok((file_name, 0))
    }

    async fn generate_file_hash(&self, s3_key: &str) -> Result<String> {
        // For S3 files, we'll need to download the file temporarily or use S3's ETag
        // For now, let's use a placeholder implementation
        // In a real implementation, you'd download the file from S3 and hash it
        log::warn!("Using placeholder hash generation for S3 file: {}", s3_key);

        // Generate a temporary hash based on the S3 key for demonstration
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(s3_key.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn find_exact_duplicates(
        &self,
        sha256_hash: &str,
        exclude_file_id: i32,
    ) -> Result<Vec<i32>> {
        let rows = sqlx::query("SELECT file_id FROM File WHERE sha256_hash = $1 AND file_id != $2")
            .bind(sha256_hash)
            .bind(exclude_file_id)
            .fetch_all(&self.db_pool)
            .await?;

        let duplicates: Vec<i32> = rows.iter().map(|row| row.get("file_id")).collect();
        Ok(duplicates)
    }

    async fn generate_file_embeddings(&self, s3_key: &str, file_name: &str) -> Result<Vec<f64>> {
        // Determine if it's an image or text file
        let is_image = self.is_image_file(file_name);

        if is_image {
            // For images, we need to get the base64 representation
            // This is a placeholder - you'd need to download from S3 and convert to base64
            let base64_content = format!("placeholder_base64_for_{}", s3_key);
            Deduplicator::generate_embeddings(
                &self.aws_profile,
                &base64_content,
                &self.bedrock_model_id,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate image embeddings: {}", e))
        } else {
            // For text files, use the filename as input (or download content from S3)
            Deduplicator::generate_embeddings(&self.aws_profile, file_name, &self.bedrock_model_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to generate text embeddings: {}", e))
        }
    }

    fn is_image_file(&self, file_name: &str) -> bool {
        let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];
        if let Some(extension) = file_name.split('.').last() {
            image_extensions.contains(&extension.to_lowercase().as_str())
        } else {
            false
        }
    }

    async fn store_embeddings_in_opensearch(
        &self,
        file_id: i32,
        file_name: &str,
        sha256_hash: &str,
        embeddings: &[f64],
    ) -> Result<()> {
        let index_name = self.get_opensearch_index(file_name);

        let document = json!({
            "file_id": file_id,
            "file_name": file_name,
            "sha256_hash": sha256_hash,
            "embedding": embeddings,
            "created_at": chrono::Utc::now().to_rfc3339()
        });

        let url = format!("{}/{}/_doc/{}", self.opensearch_url, index_name, file_id);

        let response = self
            .opensearch_client
            .put(&url)
            .json(&document)
            .send()
            .await?;

        if response.status().is_success() {
            log::info!(
                "Successfully stored embeddings for file_id: {} in index: {}",
                file_id,
                index_name
            );
        } else {
            let error_text = response.text().await?;
            log::error!("Failed to store embeddings: {}", error_text);
            return Err(anyhow::anyhow!(
                "Failed to store embeddings: {}",
                error_text
            ));
        }

        Ok(())
    }

    async fn find_similar_files(
        &self,
        embeddings: &[f64],
        exclude_file_id: i32,
        file_name: &str,
    ) -> Result<Vec<SimilarFile>> {
        let index_name = self.get_opensearch_index(file_name);

        let query = json!({
            "size": 10,
            "query": {
                "bool": {
                    "must_not": {
                        "term": { "file_id": exclude_file_id }
                    }
                }
            },
            "_source": ["file_id", "file_name", "sha256_hash"],
            "knn": {
                "embedding": {
                    "vector": embeddings,
                    "k": 10
                }
            }
        });

        let url = format!("{}/{}/_search", self.opensearch_url, index_name);

        let response = self
            .opensearch_client
            .post(&url)
            .json(&query)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            log::error!("Failed to search similar files: {}", error_text);
            return Ok(vec![]);
        }

        let search_result: serde_json::Value = response.json().await?;
        let mut similar_files = Vec::new();

        if let Some(hits) = search_result["hits"]["hits"].as_array() {
            for hit in hits {
                if let Some(score) = hit["_score"].as_f64() {
                    // Only include files with similarity score > 0.8 (adjust threshold as needed)
                    if score > 0.8 {
                        if let Some(source) = hit["_source"].as_object() {
                            similar_files.push(SimilarFile {
                                file_id: source.get("file_id").and_then(|v| v.as_i64()).unwrap_or(0)
                                    as i32,
                                file_name: source
                                    .get("file_name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                sha256_hash: source
                                    .get("sha256_hash")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                similarity_score: score,
                            });
                        }
                    }
                }
            }
        }

        Ok(similar_files)
    }

    async fn update_file_clusters(
        &self,
        file_id: i32,
        exact_duplicates: &[i32],
        similar_files: &[SimilarFile],
    ) -> Result<Option<i32>> {
        // If there are exact duplicates or similar files, create or join a cluster
        if exact_duplicates.is_empty() && similar_files.is_empty() {
            return Ok(None);
        }

        // Check if any of the similar files are already in a cluster
        let mut existing_cluster_id = None;

        for similar_file in similar_files {
            let row = sqlx::query(
                "SELECT cluster_id FROM File WHERE file_id = $1 AND cluster_id IS NOT NULL",
            )
            .bind(similar_file.file_id)
            .fetch_optional(&self.db_pool)
            .await?;

            if let Some(row) = row {
                existing_cluster_id = Some(row.get::<i32, _>("cluster_id"));
                break;
            }
        }

        let cluster_id = if let Some(cluster_id) = existing_cluster_id {
            // Join existing cluster
            cluster_id
        } else {
            // Create new cluster
            let row = sqlx::query(
                "INSERT INTO Cluster (intra_similarity_score) VALUES ($1) RETURNING cluster_id",
            )
            .bind(0.9) // Default similarity score
            .fetch_one(&self.db_pool)
            .await?;
            row.get("cluster_id")
        };

        // Update the current file's cluster
        sqlx::query("UPDATE File SET cluster_id = $1 WHERE file_id = $2")
            .bind(cluster_id)
            .bind(file_id)
            .execute(&self.db_pool)
            .await?;

        // Update similar files to join the same cluster
        for similar_file in similar_files {
            sqlx::query("UPDATE File SET cluster_id = $1 WHERE file_id = $2")
                .bind(cluster_id)
                .bind(similar_file.file_id)
                .execute(&self.db_pool)
                .await?;
        }

        Ok(Some(cluster_id))
    }

    async fn update_file_hash(&self, file_id: i32, sha256_hash: &str) -> Result<()> {
        sqlx::query("UPDATE File SET sha256_hash = $1 WHERE file_id = $2")
            .bind(sha256_hash)
            .bind(file_id)
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }
}
