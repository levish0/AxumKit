use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::{Client, Error as S3Error};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct R2Config {
    pub endpoint: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub async fn create_r2_client(config: &R2Config) -> Client {
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(config.region.clone()))
        .endpoint_url(&config.endpoint)
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "r2-credentials",
        ))
        .load()
        .await;

    let s3_config = aws_sdk_s3::config::Builder::from(&aws_config)
        .force_path_style(true)
        .build();

    Client::from_conf(s3_config)
}

#[derive(Clone)]
pub struct R2AssetsClient {
    client: Arc<Client>,
    bucket: String,
    public_domain: String,
}

impl R2AssetsClient {
    pub fn new(client: Client, bucket: String, public_domain: String) -> Self {
        Self {
            client: Arc::new(client),
            bucket,
            public_domain,
        }
    }

    pub async fn upload(&self, key: &str, body: Vec<u8>) -> Result<(), S3Error> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .send()
            .await?;
        Ok(())
    }

    pub async fn upload_with_content_type(
        &self,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<(), S3Error> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .content_type(content_type)
            .send()
            .await?;
        Ok(())
    }

    pub async fn download(
        &self,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        let data = resp.body.collect().await?;
        Ok(data.into_bytes().to_vec())
    }

    pub async fn delete(&self, key: &str) -> Result<(), S3Error> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        Ok(())
    }

    pub async fn exists(
        &self,
        key: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(err) => match &err {
                SdkError::ServiceError(service_err) => {
                    if service_err.err().is_not_found() {
                        Ok(false)
                    } else {
                        Err(Box::new(err))
                    }
                }
                _ => Err(Box::new(err)),
            },
        }
    }

    pub async fn upload_file(
        &self,
        key: &str,
        file_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let file_content = tokio::fs::read(file_path).await?;
        self.upload(key, file_content).await?;
        Ok(())
    }

    pub fn get_public_url(&self, key: &str) -> String {
        format!("{}/{}", self.public_domain, key)
    }

    pub fn get_r2_public_url(&self, key: &str) -> String {
        self.get_public_url(key)
    }
}
