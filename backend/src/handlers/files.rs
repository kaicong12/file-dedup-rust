use crate::services::files::{S3Client, S3Error};
use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct InitializeUploadRequest {
    bucket: String,
    key: String,
}

#[derive(Deserialize)]
struct CompleteUploadRequest {
    bucket: String,
    key: String,
    upload_id: String,
    parts: Vec<(i32, String)>,
}

#[derive(Serialize)]
struct UploadSuccessResponse {
    upload_id: String,
}

#[post("/upload/initiate")]
pub async fn initiate_upload(req_body: web::Json<InitializeUploadRequest>) -> impl Responder {
    let s3_client = S3Client::new("sso_profile").await;
    let multipart_result = s3_client
        .create_multipart_upload(&req_body.bucket, &req_body.key)
        .await;

    match multipart_result {
        Ok(upload_id) => HttpResponse::Ok().json(UploadSuccessResponse { upload_id }),
        Err(_) => HttpResponse::InternalServerError().json("Error Initiating multipart upload"),
    }
}

#[post("/upload/complete")]
pub async fn complete_upload(req_body: web::Json<CompleteUploadRequest>) -> impl Responder {
    let s3_client = S3Client::new("sso_profile").await;
    let complete_result = s3_client
        .complete_multipart_upload(
            &req_body.bucket,
            &req_body.key,
            req_body.upload_id.clone(),
            req_body.parts.clone(),
        )
        .await;

    match complete_result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().json("Error completing multipart upload"),
    }
}
