# File Deduplication Worker Service

This document describes the file deduplication worker service that automatically processes uploaded files to detect duplicates and similar files using SHA256 hashing and OpenSearch embeddings.

## Architecture Overview

The deduplication service consists of several components:

1. **Job Queue System** (`job_queue.rs`) - Redis-based job queue for background processing
2. **Deduplication Service** (`deduplication_service.rs`) - Core logic for processing files
3. **Worker Process** (`worker_process.rs`) - Background worker that processes jobs
4. **Deduplicator** (`deduplicator.rs`) - Utility for generating embeddings and hashes

## How It Works

### 1. File Upload Trigger

When a file is uploaded via the `/upload/complete` endpoint:

- A new record is created in the `File` table
- A deduplication job is automatically scheduled in the Redis queue
- The job contains file metadata (ID, name, S3 key, etc.)

### 2. Background Processing

The worker process continuously:

- Polls the Redis queue for new deduplication jobs
- Processes jobs one by one using the deduplication service
- Updates job status in Redis (pending → processing → completed/failed)

### 3. Deduplication Process

For each file, the service performs these steps:

#### Step 1: Generate SHA256 Hash

- Calculates SHA256 hash of the file content
- Used for exact duplicate detection

#### Step 2: Find Exact Duplicates

- Queries the database for files with the same SHA256 hash
- Identifies exact duplicates immediately

#### Step 3: Generate Embeddings

- Uses AWS Bedrock (Titan models) to generate vector embeddings
- Different handling for images vs text files:
  - **Images**: Converts to base64 and uses image embedding model
  - **Text**: Uses text content for text embedding model

#### Step 4: Store in OpenSearch

- Stores the embeddings in your AWS OpenSearch cluster
- Enables vector similarity search for near-duplicate detection

#### Step 5: Find Similar Files

- Performs k-NN search in OpenSearch to find similar files
- Uses cosine similarity with configurable threshold (default: 0.8)
- Returns files ranked by similarity score

#### Step 6: Update Clusters

- Groups similar files into clusters in the database
- Creates new clusters or joins existing ones
- Updates the `Cluster` and `File` tables accordingly

## Configuration

### Environment Variables

```bash
REDIS_URL=redis://localhost:6379
OPENSEARCH_URL=https://your-aws-opensearch-domain.region.es.amazonaws.com
OPENSEARCH_INDEX=file_embeddings
BEDROCK_MODEL_ID=amazon.titan-embed-text-v1
AWS_PROFILE=your-aws-profile
```

### Database Schema

The service uses these tables:

- `File`: Stores file metadata and SHA256 hashes
- `Cluster`: Groups similar files together
- Relationship: Files can belong to clusters (many-to-one)

## Usage Examples

### Starting the Service

The worker process starts automatically with the main application:

```rust
// In main.rs
let worker_handle = spawn_worker_process(
    pool.clone(),
    redis_url,
    opensearch_url,
    opensearch_index,
    aws_profile,
    bedrock_model_id,
).await?;
```

### Manual Job Creation

```rust
use crate::worker::{JobQueue, DeduplicationJob};

let job_queue = JobQueue::new("redis://localhost:6379")?;
let job = JobQueue::create_deduplication_job(
    file_id,
    "document.pdf".to_string(),
    "/tmp/document.pdf".to_string(),
    "uploads/document.pdf".to_string(),
);

let job_id = job_queue.enqueue_deduplication_job(job).await?;
```

### Checking Job Status

```rust
let status = job_queue.get_job_status(&job_id).await?;
match status {
    Some(status) => println!("Job status: {}", status.status),
    None => println!("Job not found"),
}
```

## Monitoring and Debugging

### Logs

The service provides detailed logging:

- Job scheduling and processing
- Embedding generation progress
- OpenSearch operations
- Error handling and retries

### Job Status Tracking

Jobs have these statuses:

- `pending`: Waiting in queue
- `processing`: Currently being processed
- `completed`: Successfully processed
- `failed`: Processing failed (with error message)

### Redis Keys

- `deduplication_jobs`: Main job queue
- `job_status:{job_id}`: Individual job status (expires after 24h)

## Error Handling

The service handles various error scenarios:

- AWS Bedrock API failures
- OpenSearch connection issues
- Database transaction failures
- Invalid file formats

Failed jobs are marked with error messages and can be retried manually if needed.

## Performance Considerations

### Scalability

- Multiple worker processes can run concurrently
- Redis queue handles job distribution
- OpenSearch provides fast similarity search

### Resource Usage

- Embedding generation requires AWS Bedrock API calls
- OpenSearch stores vector data (memory intensive)
- Database operations are optimized with indexes

### Optimization Tips

- Adjust similarity threshold based on your needs
- Monitor OpenSearch cluster performance
- Consider batch processing for high-volume scenarios

## Future Enhancements

Potential improvements:

- Support for more file types
- Advanced clustering algorithms
- Real-time duplicate detection
- Batch processing optimization
- Metrics and monitoring dashboard
