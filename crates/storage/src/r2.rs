use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::{Client, Error as S3Error};
use chrono::{DateTime, Utc};
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

    // Bound each S3 operation so a stalled/half-open connection can't hang a
    // worker handler indefinitely: `operation_attempt_timeout` bounds a single
    // HTTP attempt and `operation_timeout` bounds the whole operation across
    // retries. Without this the SDK only sets a connect timeout, so a server that
    // accepts the connection then never responds would pin the caller forever.
    let timeout_config = aws_sdk_s3::config::timeout::TimeoutConfig::builder()
        .operation_attempt_timeout(std::time::Duration::from_secs(30))
        .operation_timeout(std::time::Duration::from_secs(60))
        .build();

    let s3_config = aws_sdk_s3::config::Builder::from(&aws_config)
        .force_path_style(true)
        .timeout_config(timeout_config)
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
        object_exists(&self.client, &self.bucket, key).await
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

    pub async fn list_objects_by_prefix(&self, prefix: &str) -> Result<Vec<String>, S3Error> {
        let mut keys = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);

            if let Some(token) = continuation_token.as_deref() {
                request = request.continuation_token(token);
            }

            let resp = request.send().await?;

            keys.extend(
                resp.contents()
                    .iter()
                    .filter_map(|obj| obj.key().map(str::to_string)),
            );

            continuation_token = resp.next_continuation_token().map(str::to_string);
            if continuation_token.is_none() {
                break;
            }
        }

        Ok(keys)
    }

    pub async fn list_objects(
        &self,
        continuation_token: Option<&str>,
        max_keys: i32,
    ) -> Result<(Vec<StorageObjectInfo>, Option<String>), Box<dyn std::error::Error + Send + Sync>>
    {
        list_objects(
            &self.client,
            &self.bucket,
            None,
            continuation_token,
            max_keys,
        )
        .await
    }

    pub async fn list_objects_with_prefix(
        &self,
        prefix: &str,
        continuation_token: Option<&str>,
        max_keys: i32,
    ) -> Result<(Vec<StorageObjectInfo>, Option<String>), Box<dyn std::error::Error + Send + Sync>>
    {
        list_objects(
            &self.client,
            &self.bucket,
            Some(prefix),
            continuation_token,
            max_keys,
        )
        .await
    }
}

#[derive(Clone)]
pub struct R2RevisionClient {
    client: Arc<Client>,
    bucket: String,
}

impl R2RevisionClient {
    pub fn new(client: Client, bucket: String) -> Self {
        Self {
            client: Arc::new(client),
            bucket,
        }
    }

    pub async fn upload_content(
        &self,
        key: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let compressed = zstd::encode_all(content.as_bytes(), 3)?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(compressed.into())
            .content_type("application/zstd")
            .send()
            .await?;

        Ok(())
    }

    pub async fn download_content(
        &self,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        let data = resp.body.collect().await?;
        let bytes = data.into_bytes();

        let decompressed = zstd::decode_all(bytes.as_ref())?;
        let content = String::from_utf8(decompressed)?;

        Ok(content)
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
        object_exists(&self.client, &self.bucket, key).await
    }

    pub async fn list_objects(
        &self,
        continuation_token: Option<&str>,
        max_keys: i32,
    ) -> Result<(Vec<StorageObjectInfo>, Option<String>), Box<dyn std::error::Error + Send + Sync>>
    {
        list_objects(
            &self.client,
            &self.bucket,
            None,
            continuation_token,
            max_keys,
        )
        .await
    }
}

#[derive(Debug, Clone)]
pub struct StorageObjectInfo {
    pub key: String,
    pub last_modified: Option<DateTime<Utc>>,
}

async fn object_exists(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    match client.head_object().bucket(bucket).key(key).send().await {
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

async fn list_objects(
    client: &Client,
    bucket: &str,
    prefix: Option<&str>,
    continuation_token: Option<&str>,
    max_keys: i32,
) -> Result<(Vec<StorageObjectInfo>, Option<String>), Box<dyn std::error::Error + Send + Sync>> {
    let mut request = client.list_objects_v2().bucket(bucket).max_keys(max_keys);

    if let Some(prefix) = prefix {
        request = request.prefix(prefix);
    }

    if let Some(token) = continuation_token {
        request = request.continuation_token(token);
    }

    let resp = request.send().await?;

    let objects: Vec<StorageObjectInfo> = resp
        .contents()
        .iter()
        .filter_map(|obj| {
            let key = obj.key()?.to_string();
            let last_modified = obj
                .last_modified()
                .and_then(|t| DateTime::from_timestamp(t.secs(), t.subsec_nanos()));
            Some(StorageObjectInfo { key, last_modified })
        })
        .collect();

    let next_token = resp.next_continuation_token().map(str::to_string);

    Ok((objects, next_token))
}
