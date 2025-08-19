use aws_sdk_s3::{Client, types::Object};

#[derive(Debug)]
pub enum S3Error {
    InvalidCredentials,
    ReadFolderError,
    UploadError,
}

struct S3Client {
    client: Client,
}

impl S3Client {
    async fn new(profile_name: &str) -> Self {
        let config = aws_config::from_env()
            .profile_name(profile_name)
            .load()
            .await;

        let client = Client::new(&config);

        S3Client { client }
    }

    async fn list_files(&self, bucket: &str, folder: &str) -> Result<Vec<Object>, S3Error> {
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
}

pub async fn upload_files() -> Result<(), S3Error> {
    let profile_name = "sso_profile";
    let s3_client = S3Client::new(profile_name).await;

    let bucket_name = "mmc-did-msdora-s3-bucket";
    let folder = "documents";
    let files = s3_client.list_files(bucket_name, folder).await?;

    for file in files {
        println!(
            "File prefix: {file_name}",
            file_name = file.key().unwrap_or("Missing filename")
        );
    }

    Ok(())
}

#[cfg(test)]
mod s3_upload_tests {
    use super::*;

    #[tokio::test]
    async fn test_upload() {
        let result = upload_files().await;
        println!("Result: {result:?}");
        assert!(result.is_ok());
    }
}
