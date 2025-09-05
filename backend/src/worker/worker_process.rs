use crate::handlers::websocket::ConnectionManager;
use crate::worker::deduplication_service::DeduplicationService;
use crate::worker::job_queue::JobQueue;
use anyhow::Result;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

pub struct WorkerProcess {
    deduplication_service: DeduplicationService,
    job_queue: JobQueue,
    shutdown_signal: tokio::sync::watch::Receiver<bool>,
}

impl WorkerProcess {
    pub fn new(
        db_pool: PgPool,
        redis_url: String,
        opensearch_url: String,
        aws_profile: String,
        bedrock_model_id: String,
        shutdown_signal: tokio::sync::watch::Receiver<bool>,
        connection_manager: Option<Arc<Mutex<ConnectionManager>>>,
    ) -> Result<Self> {
        let job_queue = JobQueue::new(&redis_url)?;
        let mut deduplication_service = DeduplicationService::new(
            db_pool,
            job_queue.clone(),
            opensearch_url,
            aws_profile,
            bedrock_model_id,
        );

        // Set connection manager if provided
        if let Some(conn_mgr) = connection_manager {
            deduplication_service.set_connection_manager(conn_mgr);
        }

        Ok(WorkerProcess {
            deduplication_service,
            job_queue,
            shutdown_signal,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting worker process...");

        loop {
            // Check for shutdown signal
            if *self.shutdown_signal.borrow() {
                log::info!("Shutdown signal received, stopping worker process");
                break;
            }

            // Try to dequeue a job
            match self.job_queue.dequeue_job().await {
                Ok(Some(job)) => {
                    log::info!("Processing job: {}", job.job_id);

                    // Update job status to processing using deduplication service for WebSocket broadcasting
                    if let Err(e) = self
                        .deduplication_service
                        .update_job_status(&job.job_id, "processing", None)
                        .await
                    {
                        log::error!("Failed to update job status to processing: {}", e);
                    }

                    // Process the job
                    if let Err(e) = self
                        .deduplication_service
                        .process_deduplication_job(job)
                        .await
                    {
                        log::error!("Failed to process job: {}", e);
                    }
                }
                Ok(None) => {
                    // No jobs available, wait a bit before checking again
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    log::error!("Error dequeuing job: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }

        log::info!("Worker process stopped");
        Ok(())
    }
}

pub async fn spawn_worker_process(
    db_pool: PgPool,
    redis_url: String,
    opensearch_url: String,
    aws_profile: String,
    bedrock_model_id: String,
    connection_manager: Option<Arc<Mutex<ConnectionManager>>>,
) -> Result<tokio::task::JoinHandle<Result<()>>> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let mut worker = WorkerProcess::new(
        db_pool,
        redis_url,
        opensearch_url,
        aws_profile,
        bedrock_model_id,
        shutdown_rx,
        connection_manager,
    )?;

    let handle = tokio::spawn(async move { worker.start().await });

    // Store the shutdown sender somewhere accessible if you need graceful shutdown
    // For now, we'll just return the handle
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_process_creation() {
        // This test requires database and Redis connections
        // You might want to use test containers or mock services

        // For now, just test that the worker can be created
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://postgres:postgres@localhost:5432/file_dedup_test".to_string()
        });

        let pool = sqlx::PgPool::connect(&db_url).await;
        if pool.is_err() {
            // Skip test if database is not available
            return;
        }

        let pool = pool.unwrap();
        let redis_url = "redis://127.0.0.1:6379".to_string();
        let opensearch_url = "http://localhost:9200".to_string();
        let aws_profile = "default".to_string();
        let bedrock_model_id = "amazon.titan-embed-text-v1".to_string();

        let (_, shutdown_rx) = tokio::sync::watch::channel(false);

        let worker_result = WorkerProcess::new(
            pool,
            redis_url,
            opensearch_url,
            aws_profile,
            bedrock_model_id,
            shutdown_rx,
            None, // No connection manager for tests
        );

        assert!(worker_result.is_ok());
    }
}
