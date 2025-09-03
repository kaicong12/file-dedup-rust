use crate::config::Config;
use crate::observability::FileDeduplicationMetrics;
use crate::services::files::{MultipartUploadParams, S3Client};
use crate::worker::JobQueue;
use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::rc::Rc;
use std::time::Instant;

#[derive(Deserialize)]
struct InitializeUploadRequest {
    filename: String,
}

#[derive(Deserialize)]
struct CompleteUploadRequest {
    filename: String,
    upload_id: String,
    parts: Vec<(i32, String)>,
}

#[derive(Deserialize)]
struct PresignedUrlRequest {
    filename: String,
    expires_in_secs: Option<u64>,
    upload_id: Option<String>,
    part_number: Option<i32>,
}

#[derive(Serialize)]
struct UploadSuccessResponse {
    upload_id: String,
}

#[derive(Serialize)]
struct PresignedUrlResponse {
    presigned_url: String,
}

#[post("/upload/initiate")]
pub async fn initiate_upload(
    req_body: web::Json<InitializeUploadRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    let s3_client = S3Client::new(&config.aws_profile_name).await;
    let key = format!("{}/{}", config.s3_document_prefix, req_body.filename);

    let multipart_result = s3_client
        .create_multipart_upload(&config.s3_bucket_name, &key)
        .await;

    match multipart_result {
        Ok(upload_id) => HttpResponse::Ok().json(UploadSuccessResponse { upload_id }),
        Err(_) => HttpResponse::InternalServerError().json("Error Initiating multipart upload"),
    }
}

#[post("/upload/complete")]
pub async fn complete_upload(
    req_body: web::Json<CompleteUploadRequest>,
    config: web::Data<Config>,
    db_pool: web::Data<PgPool>,
    metrics: web::Data<Rc<FileDeduplicationMetrics>>,
) -> impl Responder {
    let s3_client = S3Client::new(&config.aws_profile_name).await;
    let key = format!("{}/{}", config.s3_document_prefix, req_body.filename);

    let complete_result = s3_client
        .complete_multipart_upload(
            &config.s3_bucket_name,
            &key,
            req_body.upload_id.clone(),
            req_body.parts.clone(),
        )
        .await;

    match complete_result {
        Ok(_) => {
            // Insert file record into database
            let insert_result = sqlx::query(
                "INSERT INTO File (file_name, sha256_hash) VALUES ($1, $2) RETURNING file_id",
            )
            .bind(&req_body.filename)
            .bind("") // Placeholder hash, will be updated by worker
            .fetch_one(db_pool.get_ref())
            .await;

            match insert_result {
                Ok(row) => {
                    let file_id: i32 = row.get("file_id");

                    // Schedule deduplication job
                    if let Ok(job_queue) = JobQueue::new(&config.redis_url) {
                        let job = JobQueue::create_deduplication_job(
                            file_id,
                            req_body.filename.clone(),
                            format!("/tmp/{}", req_body.filename), // Placeholder path
                            key,
                        );

                        match job_queue.enqueue_deduplication_job(job).await {
                            Ok(job_id) => {
                                // Increment files processed metric
                                metrics.files_processed_total.inc();

                                // Increment active jobs metric
                                metrics.active_jobs.inc();

                                log::info!(
                                    "Scheduled deduplication job {} for file_id {}",
                                    job_id,
                                    file_id
                                );
                            }
                            Err(e) => {
                                log::error!("Failed to schedule deduplication job: {}", e);
                            }
                        }
                    }

                    HttpResponse::Ok().json(serde_json::json!({
                        "message": "Upload completed successfully",
                        "file_id": file_id
                    }))
                }
                Err(e) => {
                    log::error!("Failed to insert file record: {}", e);
                    HttpResponse::InternalServerError().json("Error saving file record")
                }
            }
        }
        Err(_) => HttpResponse::InternalServerError().json("Error completing multipart upload"),
    }
}

#[post("/upload/presigned-url")]
pub async fn generate_presigned_url(
    req_body: web::Json<PresignedUrlRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    let s3_client = S3Client::new(&config.aws_profile_name).await;
    let key = format!("{}/{}", config.s3_document_prefix, req_body.filename);

    // Default expiration time is 1 hour (3600 seconds)
    let expires_in = req_body.expires_in_secs.unwrap_or(3600);

    // Create multipart params if both upload_id and part_number are provided
    let multipart_params = match (&req_body.upload_id, &req_body.part_number) {
        (Some(upload_id), Some(part_number)) => Some(MultipartUploadParams {
            upload_id: upload_id.clone(),
            part: *part_number,
        }),
        _ => None,
    };

    let presigned_result = s3_client
        .generate_presigned_upload_url(&config.s3_bucket_name, &key, expires_in, multipart_params)
        .await;

    match presigned_result {
        Ok(presigned_url) => HttpResponse::Ok().json(PresignedUrlResponse { presigned_url }),
        Err(_) => HttpResponse::InternalServerError().json("Error generating presigned URL"),
    }
}
