use application::ApplicationError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ErrorDescription {
    pub code: &'static str,
    pub message: &'static str,
}
#[derive(Serialize, ToSchema)]
pub struct ErrorOutput {
    pub error: ErrorDescription,
    pub request_id: Uuid,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub description: ErrorDescription,
    retry_after_seconds: Option<u64>,
}
impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            description: ErrorDescription { code, message },
            retry_after_seconds: None,
        }
    }
    pub fn invalid_input() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "invalid_input",
            "The request is invalid.",
        )
    }
}
impl From<ApplicationError> for ApiError {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::InvalidInput(message) => {
                Self::new(StatusCode::BAD_REQUEST, "invalid_input", message)
            }
            ApplicationError::Unauthenticated => Self::new(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Authentication is required.",
            ),
            ApplicationError::Forbidden => Self::new(
                StatusCode::FORBIDDEN,
                "forbidden",
                "This action is not allowed.",
            ),
            ApplicationError::NotFound => Self::new(
                StatusCode::NOT_FOUND,
                "not_found",
                "The resource was not found.",
            ),
            ApplicationError::Conflict => Self::new(
                StatusCode::CONFLICT,
                "conflict",
                "The operation conflicts with the current state.",
            ),
            ApplicationError::RateLimited => Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "Too many attempts. Try again later.",
            ),
            ApplicationError::ApiRateLimited {
                retry_after_seconds,
            } => {
                let mut result = Self::new(
                    StatusCode::TOO_MANY_REQUESTS,
                    "api_rate_limited",
                    "The API client's minute allowance is exhausted.",
                );
                result.retry_after_seconds = Some(retry_after_seconds);
                result
            }
            ApplicationError::ApiQuotaExceeded {
                retry_after_seconds,
            } => {
                let mut result = Self::new(
                    StatusCode::TOO_MANY_REQUESTS,
                    "api_quota_exceeded",
                    "The API client's monthly allowance is exhausted.",
                );
                result.retry_after_seconds = Some(retry_after_seconds);
                result
            }
            ApplicationError::ExternalUnavailable => {
                tracing::error!(error_code = "provider_unavailable", "request failed");
                Self::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "provider_unavailable",
                    "The identity provider is temporarily unavailable. Start a new login flow.",
                )
            }
            ApplicationError::IntegrationUnavailable => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "integration_unavailable",
                "The integration is temporarily unavailable.",
            ),
            ApplicationError::HumanVerificationFailed => Self::new(
                StatusCode::FORBIDDEN,
                "human_verification_failed",
                "Human verification failed. Complete a new challenge and try again.",
            ),
            ApplicationError::HumanVerificationUnavailable => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "human_verification_unavailable",
                "Human verification is temporarily unavailable.",
            ),
            ApplicationError::FileTooLarge => Self::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                "The file exceeds the upload limit.",
            ),
            ApplicationError::FileQuotaExceeded => Self::new(
                StatusCode::CONFLICT,
                "file_quota_exceeded",
                "The organization file quota is exhausted.",
            ),
            ApplicationError::TransferTimeout => Self::new(
                StatusCode::REQUEST_TIMEOUT,
                "transfer_timeout",
                "The file transfer timed out.",
            ),
            ApplicationError::Storage(_) | ApplicationError::Job(_) => {
                tracing::error!(error_code = "storage_unavailable", "request failed");
                Self::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "unavailable",
                    "The service is temporarily unavailable.",
                )
            }
            ApplicationError::Internal(_) => {
                tracing::error!(error_code = "internal_error", "request failed");
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "The operation failed.",
                )
            }
        }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = self.status.into_response();
        response.extensions_mut().insert(self.description);
        if self.status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_static("Bearer realm=\"api\""),
            );
        }
        if let Some(seconds) = self.retry_after_seconds {
            response.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_str(&seconds.to_string()).expect("integer header"),
            );
        }
        response
    }
}
