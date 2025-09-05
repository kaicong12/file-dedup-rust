use crate::config::Config;
use crate::handlers::jobs::create_job_record;
use crate::metrics::DeduplicationMetrics;
use crate::services::files::{MultipartUploadParams, S3Client};
use crate::worker::JobQueue;
use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Helper function to determine if a file is an image based on its extension
fn is_image_file(file_name: &str) -> bool {
    let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];
    if let Some(extension) = file_name.split('.').last() {
        image_extensions.contains(&extension.to_lowercase().as_str())
    } else {
        false
    }
}

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
    metrics: web::Data<Arc<DeduplicationMetrics>>,
) -> impl Responder {
    let s3_client = S3Client::new(&config.aws_profile_name).await;
    let key = format!("{}/{}", config.s3_document_prefix, req_body.filename);

    // Start timing S3 operation
    let s3_timer = crate::metrics::MetricsTimer::new("s3_complete_upload".to_string());

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
            // Record successful S3 operation
            s3_timer.finish_s3(&metrics, "complete_multipart_upload");

            // Determine file type for metrics
            let file_type = if is_image_file(&req_body.filename) {
                "image"
            } else {
                "text"
            };

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

                    // Record file upload metrics
                    metrics.record_file_processed(file_type, 0); // File size unknown for now

                    // Schedule deduplication job
                    if let Ok(job_queue) = JobQueue::new(&config.redis_url) {
                        let job = JobQueue::create_deduplication_job(
                            file_id,
                            req_body.filename.clone(),
                            format!("/tmp/{}", req_body.filename), // Placeholder path
                            key.clone(),
                        );

                        match job_queue.enqueue_deduplication_job(job.clone()).await {
                            Ok(job_id) => {
                                // Parse job_id as UUID for database
                                if let Ok(job_uuid) = Uuid::parse_str(&job_id) {
                                    // Create job record in database
                                    if let Err(e) = create_job_record(
                                        db_pool.get_ref(),
                                        job_uuid,
                                        file_id,
                                        &req_body.filename,
                                        Some(&format!("/tmp/{}", req_body.filename)),
                                        &key,
                                    )
                                    .await
                                    {
                                        log::error!(
                                            "Failed to create job record in database: {}",
                                            e
                                        );
                                    }
                                }

                                log::info!(
                                    "Scheduled deduplication job {} for file_id {}",
                                    job_id,
                                    file_id
                                );

                                return HttpResponse::Ok().json(serde_json::json!({
                                    "message": "Upload completed successfully",
                                    "file_id": file_id,
                                    "job_id": job_id
                                }));
                            }
                            Err(e) => {
                                log::error!("Failed to schedule deduplication job: {}", e);
                            }
                        }
                    }

                    HttpResponse::Ok().json(serde_json::json!({
                        "message": "Upload completed successfully",
                        "file_id": file_id,
                        "job_id": null
                    }))
                }
                Err(e) => {
                    log::error!("Failed to insert file record: {}", e);
                    HttpResponse::InternalServerError().json("Error saving file record")
                }
            }
        }
        Err(_) => {
            // Record S3 error
            metrics.record_s3_error("complete_multipart_upload");
            HttpResponse::InternalServerError().json("Error completing multipart upload")
        }
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
