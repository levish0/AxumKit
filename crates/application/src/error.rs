#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("authentication required")]
    Unauthenticated,
    #[error("access denied")]
    Forbidden,
    #[error("resource not found")]
    NotFound,
    #[error("conflicting state")]
    Conflict,
    #[error("too many attempts")]
    RateLimited,
    #[error("API client request rate exceeded")]
    ApiRateLimited { retry_after_seconds: u64 },
    #[error("API client monthly quota exceeded")]
    ApiQuotaExceeded { retry_after_seconds: u64 },
    #[error("external service unavailable")]
    ExternalUnavailable,
    #[error("integration unavailable")]
    IntegrationUnavailable,
    #[error("human verification failed")]
    HumanVerificationFailed,
    #[error("human verification unavailable")]
    HumanVerificationUnavailable,
    #[error("file is too large")]
    FileTooLarge,
    #[error("file quota exceeded")]
    FileQuotaExceeded,
    #[error("transfer timed out")]
    TransferTimeout,
    #[error("storage unavailable")]
    Storage(#[from] sea_orm::DbErr),
    #[error("job registration failed")]
    Job(#[from] jobs::JobError),
    #[error("internal operation failed")]
    Internal(#[source] anyhow::Error),
}
pub type ApplicationResult<T> = Result<T, ApplicationError>;

impl ApplicationError {
    pub fn internal(error: impl Into<anyhow::Error>) -> Self {
        Self::Internal(error.into())
    }
}
