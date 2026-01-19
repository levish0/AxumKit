use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::{Client, Error as S3Error};
use axumkit_config::ServerConfig;
use std::sync::Arc;
use tracing::info;

/// SeaweedFS 클라이언트 (S3 호환 API 사용)
/// revision 콘텐츠 저장용
#[derive(Clone)]
pub struct SeaweedFsClient {
    client: Arc<Client>,
    bucket: String,
}

impl SeaweedFsClient {
    pub fn new(client: Client, bucket: String) -> Self {
        Self {
            client: Arc::new(client),
            bucket,
        }
    }

    /// 콘텐츠 업로드 (zstd 압축 적용)
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

    /// 콘텐츠 다운로드 (zstd 압축 해제)
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

    /// raw 바이트 업로드 (압축 없음)
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

    /// raw 바이트 다운로드
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

    /// 오브젝트 삭제
    pub async fn delete(&self, key: &str) -> Result<(), S3Error> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        Ok(())
    }

    /// 오브젝트 존재 여부 확인
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
}

/// 버킷 이름 (하드코딩 - 변경할 이유 없음)
const BUCKET_NAME: &str = "axumkit-content";

pub async fn establish_seaweedfs_connection()
-> Result<SeaweedFsClient, Box<dyn std::error::Error + Send + Sync>> {
    let config = ServerConfig::get();

    info!("Connecting to SeaweedFS at: {}", config.seaweedfs_endpoint);

    // SeaweedFS S3 API - 내부 네트워크, 인증 없음
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .endpoint_url(&config.seaweedfs_endpoint)
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            "",
            "",
            None,
            None,
            "anonymous",
        ))
        .load()
        .await;

    let s3_config = aws_sdk_s3::config::Builder::from(&aws_config)
        .force_path_style(true)
        .build();

    let client = Client::from_conf(s3_config);
    let seaweedfs_client = SeaweedFsClient::new(client, BUCKET_NAME.to_string());

    // 버킷 생성 (없으면)
    if seaweedfs_client
        .client
        .head_bucket()
        .bucket(BUCKET_NAME)
        .send()
        .await
        .is_err()
    {
        info!("Creating SeaweedFS bucket: {}", BUCKET_NAME);
        seaweedfs_client
            .client
            .create_bucket()
            .bucket(BUCKET_NAME)
            .send()
            .await?;
    }

    info!("Successfully connected to SeaweedFS");
    Ok(seaweedfs_client)
}
