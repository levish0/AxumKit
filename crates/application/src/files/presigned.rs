use super::{
    DownloadMethod, PresignedTransfer, TransferHeader, TransferMethod, storage::FileStorage,
};
use crate::{ApplicationError, ApplicationResult};
use aws_sdk_s3::{
    presigning::{PresignedRequest, PresigningConfig},
    types::{CorsConfiguration, CorsRule},
};
use chrono::{DateTime, Utc};

fn signing_config(
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> ApplicationResult<PresigningConfig> {
    let lifetime = (expires_at - issued_at)
        .to_std()
        .ok()
        .filter(|lifetime| !lifetime.is_zero())
        .ok_or(ApplicationError::Conflict)?;
    PresigningConfig::builder()
        .start_time(issued_at.into())
        .expires_in(lifetime)
        .build()
        .map_err(ApplicationError::internal)
}

fn transfer(
    request: PresignedRequest,
    method: TransferMethod,
    expires_at: DateTime<Utc>,
) -> PresignedTransfer {
    PresignedTransfer {
        method,
        url: request.uri().to_owned(),
        headers: request
            .headers()
            // Fetch forbids setting Content-Length; File/Blob determines its signed value.
            .filter(|(name, _)| !name.eq_ignore_ascii_case("content-length"))
            .map(|(name, value)| TransferHeader {
                name: name.to_owned(),
                value: value.to_owned(),
            })
            .collect(),
        expires_at,
    }
}

fn disposition(filename: &str) -> String {
    let encoded =
        percent_encoding::utf8_percent_encode(filename, percent_encoding::NON_ALPHANUMERIC);
    format!("attachment; filename=\"attachment\"; filename*=UTF-8''{encoded}")
}

impl FileStorage {
    pub(super) async fn configure_browser_transfers(
        &self,
        origins: &[String],
    ) -> ApplicationResult<()> {
        if origins.is_empty() {
            return Ok(());
        }
        let mut rule = CorsRule::builder()
            .allowed_methods("PUT")
            .allowed_methods("GET")
            .allowed_methods("HEAD")
            .expose_headers("ETag")
            .expose_headers("Content-Range")
            .expose_headers("Content-Disposition")
            .expose_headers("Accept-Ranges")
            .max_age_seconds(600);
        for name in [
            "content-type",
            "content-encoding",
            "content-disposition",
            "cache-control",
            "if-none-match",
            "range",
            "if-range",
        ] {
            rule = rule.allowed_headers(name);
        }
        for origin in origins {
            rule = rule.allowed_origins(origin);
        }
        self.client
            .put_bucket_cors()
            .bucket(&self.bucket)
            .cors_configuration(
                CorsConfiguration::builder()
                    .cors_rules(rule.build().map_err(ApplicationError::internal)?)
                    .build()
                    .map_err(ApplicationError::internal)?,
            )
            .send()
            .await
            .map_err(|_| ApplicationError::IntegrationUnavailable)?;
        Ok(())
    }

    pub(super) async fn presign_upload(
        &self,
        key: &str,
        filename: &str,
        size: u64,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> ApplicationResult<PresignedTransfer> {
        let request = self
            .signing_client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_length(size as i64)
            .content_type("application/octet-stream")
            .content_encoding("identity")
            .content_disposition(disposition(filename))
            .cache_control("no-store")
            .if_none_match("*")
            .presigned(signing_config(issued_at, expires_at)?)
            .await
            .map_err(|_| ApplicationError::IntegrationUnavailable)?;
        Ok(transfer(request, TransferMethod::Put, expires_at))
    }

    pub(super) async fn presign_download(
        &self,
        key: &str,
        filename: &str,
        method: DownloadMethod,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> ApplicationResult<PresignedTransfer> {
        let config = signing_config(issued_at, expires_at)?;
        let request = match method {
            DownloadMethod::Get => self
                .signing_client
                .get_object()
                .bucket(&self.bucket)
                .key(key)
                .response_content_type("application/octet-stream")
                .response_content_encoding("identity")
                .response_content_disposition(disposition(filename))
                .response_cache_control("no-store")
                .presigned(config)
                .await
                .map_err(|_| ApplicationError::IntegrationUnavailable)?,
            DownloadMethod::Head => self
                .signing_client
                .head_object()
                .bucket(&self.bucket)
                .key(key)
                .presigned(config)
                .await
                .map_err(|_| ApplicationError::IntegrationUnavailable)?,
        };
        let method = match method {
            DownloadMethod::Get => TransferMethod::Get,
            DownloadMethod::Head => TransferMethod::Head,
        };
        Ok(transfer(request, method, expires_at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::SecretString, files::S3Settings};

    #[tokio::test]
    async fn sdk_signatures_bind_body_metadata_and_the_client_endpoint() {
        let storage = FileStorage::new(&S3Settings {
            endpoint: "http://internal-storage:8333/".parse().unwrap(),
            public_endpoint: Some("https://files.example.test/".parse().unwrap()),
            bucket: "test-originals".into(),
            region: "us-east-1".into(),
            access_key: SecretString::new("test-access"),
            secret_key: SecretString::new("test-secret"),
            allow_http: true,
        });
        let issued_at = Utc::now();
        let expires_at = issued_at + chrono::Duration::seconds(300);
        let upload = storage
            .presign_upload(
                "originals/one",
                "image \"한글.svg",
                3,
                issued_at,
                expires_at,
            )
            .await
            .unwrap();
        let url = url::Url::parse(&upload.url).unwrap();
        assert_eq!(url.host_str(), Some("files.example.test"));
        assert_eq!(url.path(), "/test-originals/originals/one");
        let query = url
            .query_pairs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(query["X-Amz-Expires"], "300");
        for header in [
            "content-length",
            "content-type",
            "content-encoding",
            "content-disposition",
            "cache-control",
            "if-none-match",
        ] {
            assert!(
                query["X-Amz-SignedHeaders"]
                    .split(';')
                    .any(|name| name == header)
            );
        }
        assert!(
            upload
                .headers
                .iter()
                .all(|header| header.name != "content-length")
        );
        assert!(!format!("{upload:?}").contains("X-Amz"));
        let head = storage
            .presign_download(
                "originals/one",
                "image.svg",
                DownloadMethod::Head,
                issued_at,
                expires_at,
            )
            .await
            .unwrap();
        assert_eq!(head.method.as_str(), "HEAD");
        assert!(matches!(
            signing_config(issued_at, issued_at),
            Err(ApplicationError::Conflict)
        ));
    }
}
