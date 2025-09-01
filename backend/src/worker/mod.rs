pub mod deduplication_service;
pub mod deduplicator;
pub mod job_queue;
pub mod worker_process;

pub use deduplication_service::{DeduplicationResult, DeduplicationService, SimilarFile};
pub use deduplicator::Deduplicator;
pub use job_queue::{DeduplicationJob, JobQueue, JobStatus};
pub use worker_process::{WorkerProcess, spawn_worker_process};
