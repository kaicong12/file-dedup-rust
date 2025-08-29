use crate::config::Config;
use crate::services::files::{MultipartUploadParams, S3Client};
use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};

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
        Ok(_) => HttpResponse::Ok().finish(),
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
