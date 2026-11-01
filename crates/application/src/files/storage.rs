use crate::{ApplicationError, ApplicationResult, config::SecretString};
use aws_sdk_s3::{
    Client,
    config::{
        BehaviorVersion, Credentials, Region, RequestChecksumCalculation,
        ResponseChecksumValidation, retry::RetryConfig, timeout::TimeoutConfig,
    },
};
use std::time::Duration;
use url::Url;

#[derive(Clone, Debug)]
pub struct S3Settings {
    pub endpoint: Url,
    pub public_endpoint: Option<Url>,
    pub bucket: String,
    pub region: String,
    pub access_key: SecretString,
    pub secret_key: SecretString,
    pub allow_http: bool,
}

impl S3Settings {
    pub fn validate(&self) -> ApplicationResult<()> {
        if let Some(endpoint) = &self.public_endpoint {
            let mut public = self.clone();
            public.endpoint = endpoint.clone();
            public.public_endpoint = None;
            public.validate()?;
        }
        if !(self.endpoint.scheme() == "https"
            || (self.allow_http && self.endpoint.scheme() == "http"))
            || self.endpoint.host_str().is_none()
            || !self.endpoint.username().is_empty()
            || self.endpoint.password().is_some()
            || self.endpoint.query().is_some()
            || self.endpoint.fragment().is_some()
            || self.bucket.is_empty()
            || self.bucket.len() > 63
            || !self.bucket.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'.'
            })
            || self.region.is_empty()
            || self.access_key.expose().is_empty()
            || self.secret_key.expose().is_empty()
        {
            return Err(ApplicationError::InvalidInput("invalid S3 settings"));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(super) struct FileStorage {
    pub(super) client: Client,
    pub(super) signing_client: Client,
    pub(super) bucket: String,
}

impl FileStorage {
    pub fn new(settings: &S3Settings) -> Self {
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(settings.endpoint.as_str())
            .region(Region::new(settings.region.clone()))
            .credentials_provider(Credentials::new(
                settings.access_key.expose(),
                settings.secret_key.expose(),
                None,
                None,
                "explicit-s3-settings",
            ))
            .force_path_style(true)
            .retry_config(RetryConfig::standard().with_max_attempts(2))
            .timeout_config(
                TimeoutConfig::builder()
                    .connect_timeout(Duration::from_secs(5))
                    .operation_timeout(Duration::from_secs(60))
                    .build(),
            )
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .build();
        Self {
            client: Client::from_conf(config.clone()),
            signing_client: Client::from_conf(
                config
                    .to_builder()
                    .endpoint_url(
                        settings
                            .public_endpoint
                            .as_ref()
                            .unwrap_or(&settings.endpoint)
                            .as_str(),
                    )
                    .build(),
            ),
            bucket: settings.bucket.clone(),
        }
    }

    pub async fn delete_original(&self, key: &str) -> ApplicationResult<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|_| ApplicationError::IntegrationUnavailable)?;
        Ok(())
    }

    pub async fn require_original_metadata(
        &self,
        key: &str,
        expected_bytes: u64,
    ) -> ApplicationResult<()> {
        let response = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| {
                if error
                    .as_service_error()
                    .is_some_and(|error| error.is_not_found())
                {
                    ApplicationError::Conflict
                } else {
                    ApplicationError::IntegrationUnavailable
                }
            })?;
        if response.content_length() != Some(expected_bytes as i64)
            || response.content_type() != Some("application/octet-stream")
            || response.content_encoding() != Some("identity")
        {
            return Err(ApplicationError::InvalidInput(
                "uploaded object does not match reservation",
            ));
        }
        Ok(())
    }

    pub async fn create_bucket(&self) -> ApplicationResult<()> {
        if self
            .client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .is_ok()
        {
            return Ok(());
        }
        self.client
            .create_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map_err(|_| ApplicationError::IntegrationUnavailable)?;
        Ok(())
    }
}
