use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::{Client, presigning::PresigningConfig, types::Object};
use std::time::Duration;

#[derive(Debug)]
pub enum S3Error {
    InvalidCredentials,
    ReadFolderError,
    UploadError,
}

pub struct S3Client {
    client: Client,
}

pub struct MultipartUploadParams {
    pub upload_id: String,
    pub part: i32,
}

type S3Result<T> = Result<T, S3Error>;

impl S3Client {
    pub async fn new(profile_name: &str) -> Self {
        let config = aws_config::from_env()
            .profile_name(profile_name)
            .load()
            .await;

        let client = Client::new(&config);

        S3Client { client }
    }

    pub async fn list_files(&self, bucket: &str, folder: &str) -> Result<Vec<Object>, S3Error> {
        let objects = self
            .client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(folder)
            .send()
            .await
            .map_err(|_| S3Error::ReadFolderError)?;

        match objects.contents {
            Some(files) => Ok(files),
            None => Ok(vec![]),
        }
    }

    pub async fn generate_presigned_upload_url(
        &self,
        bucket: &str,
        key: &str,
        expires_in_secs: u64,
        multipart_params: Option<MultipartUploadParams>,
    ) -> S3Result<String> {
        // presigned url is basically a URL which the client can use to upload files
        // wihout needing access to AWS credentials
        let presign_config = PresigningConfig::expires_in(Duration::from_secs(expires_in_secs))
            .map_err(|_| S3Error::UploadError)?;

        let presigned_req = if let Some(params) = multipart_params {
            // Multipart upload: generate presigned URL for a part
            self.client
                .upload_part()
                .bucket(bucket)
                .key(key)
                .upload_id(params.upload_id)
                .part_number(params.part)
                .presigned(presign_config)
                .await
                .map_err(|_| S3Error::UploadError)?
        } else {
            // Single-part upload: generate presigned URL for the whole object
            self.client
                .put_object()
                .bucket(bucket)
                .key(key)
                .presigned(presign_config)
                .await
                .map_err(|_| S3Error::UploadError)?
        };

        Ok(presigned_req.uri().to_string())
    }

    // Multipart upload is a three-step process:
    // 1. You initiate the upload,
    // 2. upload the object parts,
    // 3. and—after you've uploaded all the parts—complete the multipart upload.
    pub async fn create_multipart_upload(&self, bucket: &str, key: &str) -> S3Result<String> {
        let resp = self
            .client
            .create_multipart_upload()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|_| S3Error::UploadError)?;

        resp.upload_id()
            .map(|s| s.to_string())
            .ok_or(S3Error::UploadError)
    }

    pub async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: String,
        parts: Vec<(i32, String)>,
    ) -> S3Result<()> {
        // eTag of each single-part upload can be fetched from the response header
        let completed_parts = parts
            .into_iter()
            .map(|(part_number, etag)| {
                CompletedPart::builder()
                    .part_number(part_number)
                    .e_tag(etag)
                    .build()
            })
            .collect::<Vec<_>>();

        self.client
            .complete_multipart_upload()
            .bucket(bucket)
            .key(key)
            .multipart_upload(
                CompletedMultipartUpload::builder()
                    .set_parts(Some(completed_parts))
                    .build(),
            )
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|_| S3Error::UploadError)?;

        Ok(())
    }
}

#[cfg(test)]
mod s3_upload_tests {
    use super::*;

    #[tokio::test]
    async fn test_upload() -> Result<(), S3Error> {
        let profile_name = "sso_profile";
        let s3_client = S3Client::new(profile_name).await;

        let bucket_name = "mmc-did-msdora-s3-bucket";
        let folder = "documents";
        let files = s3_client.list_files(bucket_name, folder).await?;

        assert!(files.len() >= 4);

        Ok(())
    }
}
