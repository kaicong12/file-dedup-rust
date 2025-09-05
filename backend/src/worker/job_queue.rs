use anyhow::Result;
use redis::{Client, Commands, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeduplicationJob {
    pub job_id: String,
    pub file_id: i32,
    pub file_name: String,
    pub file_path: String,
    pub s3_key: String,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobStatus {
    pub job_id: String,
    pub status: String, // "pending", "processing", "completed", "failed"
    pub created_at: u64,
    pub updated_at: u64,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct JobQueue {
    redis_client: Client,
}

impl JobQueue {
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        Ok(JobQueue {
            redis_client: client,
        })
    }

    pub fn get_connection(&self) -> Result<Connection> {
        Ok(self.redis_client.get_connection()?)
    }

    pub async fn enqueue_deduplication_job(&self, job: DeduplicationJob) -> Result<String> {
        let mut conn = self.get_connection()?;

        // Serialize the job
        let job_data = serde_json::to_string(&job)?;

        // Add to the job queue
        let _: () = conn.lpush("deduplication_jobs", &job_data)?;

        // Store job status as pending
        self.update_job_status(&job.job_id, "pending", None).await?;

        log::info!("Enqueued deduplication job: {}", job.job_id);
        Ok(job.job_id)
    }

    pub async fn dequeue_job(&self) -> Result<Option<DeduplicationJob>> {
        let mut conn = self.get_connection()?;

        // Block for up to 5 seconds waiting for a job
        let result: Option<Vec<String>> = conn.brpop("deduplication_jobs", 5.0)?;

        if let Some(job_data) = result {
            if job_data.len() >= 2 {
                let job: DeduplicationJob = serde_json::from_str(&job_data[1])?;

                // Update job status to processing
                self.update_job_status(&job.job_id, "processing", None)
                    .await?;

                return Ok(Some(job));
            }
        }

        Ok(None)
    }

    pub async fn update_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<String>,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;

        let status_key = format!("job_status:{}", job_id);

        // Get current status to preserve created_at
        let current_status: Option<String> = conn.get(&status_key)?;
        let created_at = if let Some(current_data) = current_status {
            let current: JobStatus = serde_json::from_str(&current_data)?;
            current.created_at
        } else {
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
        };

        let updated_status = JobStatus {
            job_id: job_id.to_string(),
            status: status.to_string(),
            created_at,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            error_message,
        };

        let status_data = serde_json::to_string(&updated_status)?;
        let _: () = conn.set(&status_key, &status_data)?;

        log::info!("Updated job {} status to: {}", job_id, status);
        Ok(())
    }

    pub async fn get_job_status(&self, job_id: &str) -> Result<Option<JobStatus>> {
        let mut conn = self.get_connection()?;

        let status_key = format!("job_status:{}", job_id);
        let status_data: Option<String> = conn.get(&status_key)?;

        if let Some(data) = status_data {
            let status: JobStatus = serde_json::from_str(&data)?;
            Ok(Some(status))
        } else {
            Ok(None)
        }
    }

    pub fn create_deduplication_job(
        file_id: i32,
        file_name: String,
        file_path: String,
        s3_key: String,
    ) -> DeduplicationJob {
        DeduplicationJob {
            job_id: Uuid::new_v4().to_string(),
            file_id,
            file_name,
            file_path,
            s3_key,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_queue_operations() {
        // This test requires Redis to be running
        let redis_url = "redis://127.0.0.1:6379";
        let queue = JobQueue::new(redis_url).unwrap();

        let job = JobQueue::create_deduplication_job(
            1,
            "test_file.txt".to_string(),
            "/tmp/test_file.txt".to_string(),
            "uploads/test_file.txt".to_string(),
        );

        // Test enqueue
        let job_id = queue.enqueue_deduplication_job(job.clone()).await.unwrap();
        assert_eq!(job_id, job.job_id);

        // Test status check
        let status = queue.get_job_status(&job_id).await.unwrap();
        assert!(status.is_some());
        assert_eq!(status.unwrap().status, "pending");

        // Test dequeue
        let dequeued_job = queue.dequeue_job().await.unwrap();
        assert!(dequeued_job.is_some());
        assert_eq!(dequeued_job.unwrap().job_id, job_id);

        // Test status update
        queue
            .update_job_status(&job_id, "completed", None)
            .await
            .unwrap();
        let updated_status = queue.get_job_status(&job_id).await.unwrap();
        assert!(updated_status.is_some());
        assert_eq!(updated_status.unwrap().status, "completed");
    }
}
