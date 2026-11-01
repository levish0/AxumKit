use super::{S3Settings, SearchSettings, entities::file};
use crate::{ApplicationError, ApplicationResult, files::FileState};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct FileSettings {
    pub storage: Option<S3Settings>,
    pub search: Option<SearchSettings>,
    pub max_file_bytes: u64,
    pub organization_quota_bytes: u64,
    pub organization_file_limit: u64,
    pub pending_file_limit: u64,
    pub reservations_per_minute: u64,
    pub tombstone_days: i64,
    pub upload_url_seconds: u64,
    pub download_url_seconds: u64,
    pub upload_grace_seconds: u64,
    pub browser_origins: Vec<String>,
}

impl Default for FileSettings {
    fn default() -> Self {
        Self {
            storage: None,
            search: None,
            max_file_bytes: 100 * 1024 * 1024,
            organization_quota_bytes: 1024 * 1024 * 1024,
            organization_file_limit: 10_000,
            pending_file_limit: 100,
            reservations_per_minute: 30,
            tombstone_days: 30,
            upload_url_seconds: 300,
            download_url_seconds: 60,
            upload_grace_seconds: 3600,
            browser_origins: Vec::new(),
        }
    }
}

impl FileSettings {
    pub fn validate(&self) -> ApplicationResult<()> {
        if !(1..=1024 * 1024 * 1024).contains(&self.max_file_bytes)
            || self.organization_quota_bytes < self.max_file_bytes
            || self.organization_quota_bytes > i64::MAX as u64
            || !(1..=1_000_000).contains(&self.organization_file_limit)
            || !(1..=self.organization_file_limit).contains(&self.pending_file_limit)
            || !(1..=10_000).contains(&self.reservations_per_minute)
            || !(1..=3650).contains(&self.tombstone_days)
            || !(1..=3600).contains(&self.upload_url_seconds)
            || !(1..=300).contains(&self.download_url_seconds)
            || !(30..=86400).contains(&self.upload_grace_seconds)
        {
            return Err(ApplicationError::InvalidInput("invalid file settings"));
        }
        if let Some(storage) = &self.storage {
            storage.validate()?;
        }
        if let Some(search) = &self.search {
            search.validate()?;
        }
        for origin in &self.browser_origins {
            let url = url::Url::parse(origin)
                .map_err(|_| ApplicationError::InvalidInput("invalid storage browser origin"))?;
            if !matches!(url.scheme(), "http" | "https")
                || url.origin().ascii_serialization() != *origin
            {
                return Err(ApplicationError::InvalidInput(
                    "invalid storage browser origin",
                ));
            }
        }
        Ok(())
    }
}

pub struct ReserveFile {
    pub filename: String,
    pub media_type: Option<String>,
    pub byte_size: u64,
}

pub struct FileSummary {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub uploader_id: Uuid,
    pub filename: String,
    pub media_type: Option<String>,
    pub byte_size: u64,
    pub state: FileState,
    pub version: i64,
    pub content_hash: Option<String>,
    pub upload_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<file::Model> for FileSummary {
    fn from(file: file::Model) -> Self {
        Self {
            id: file.id,
            organization_id: file.organization_id,
            uploader_id: file.uploader_id,
            filename: file.filename,
            media_type: file.media_type,
            byte_size: file.byte_size as u64,
            state: file.state,
            version: file.version,
            content_hash: file.content_hash,
            upload_expires_at: file.upload_expires_at,
            created_at: file.created_at,
        }
    }
}

#[derive(Clone, Copy)]
pub enum DownloadMethod {
    Get,
    Head,
}

#[derive(Clone, Copy)]
pub enum TransferMethod {
    Put,
    Get,
    Head,
}

impl TransferMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Put => "PUT",
            Self::Get => "GET",
            Self::Head => "HEAD",
        }
    }
}

pub struct PresignedTransfer {
    pub url: String,
    pub method: TransferMethod,
    /// Browser clients set Content-Length from the exact File/Blob body.
    pub headers: Vec<TransferHeader>,
    pub expires_at: DateTime<Utc>,
}

impl std::fmt::Debug for PresignedTransfer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PresignedTransfer")
            .field("url", &"[REDACTED]")
            .field("method", &self.method.as_str())
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

pub struct TransferHeader {
    pub name: String,
    pub value: String,
}
