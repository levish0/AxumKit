use crate::config::WorkerConfig;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::Client;
use aws_sdk_s3::config::RequestChecksumCalculation;
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

    /// 오브젝트 삭제
    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        Ok(())
    }

    /// 오브젝트 목록 조회 (페이지네이션)
    /// Returns (keys, continuation_token)
    pub async fn list_objects(
        &self,
        continuation_token: Option<&str>,
        max_keys: i32,
    ) -> Result<(Vec<StorageObjectInfo>, Option<String>), Box<dyn std::error::Error + Send + Sync>>
    {
        let mut request = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .max_keys(max_keys);

        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }

        let resp = request.send().await?;

        let objects: Vec<StorageObjectInfo> = resp
            .contents()
            .iter()
            .filter_map(|obj| {
                let key = obj.key()?.to_string();
                let last_modified = obj.last_modified().map(|t| {
                    chrono::DateTime::from_timestamp(t.secs(), t.subsec_nanos()).unwrap_or_default()
                });
                Some(StorageObjectInfo { key, last_modified })
            })
            .collect();

        let next_token = resp.next_continuation_token().map(|s| s.to_string());

        Ok((objects, next_token))
    }
}

/// Storage object info for cleanup
#[derive(Debug, Clone)]
pub struct StorageObjectInfo {
    pub key: String,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

const BUCKET_NAME: &str = "axumkit-content";

pub async fn establish_seaweedfs_connection(
    config: &WorkerConfig,
) -> anyhow::Result<SeaweedFsClient> {
    info!("Connecting to SeaweedFS at: {}", config.seaweedfs_endpoint);

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
        .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
        .build();

    let client = Client::from_conf(s3_config);
    let seaweedfs_client = SeaweedFsClient::new(client, BUCKET_NAME.to_string());

    info!("Successfully connected to SeaweedFS");
    Ok(seaweedfs_client)
}
