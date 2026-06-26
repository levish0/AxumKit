use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use config::ServerConfig;
use sea_orm::{DbErr, TransactionError};
use serde::Serialize;
use tracing::error;
use utoipa::ToSchema;

pub type ServiceResult<T> = Result<T, Errors>;
type ErrorMapping = (StatusCode, &'static str, Option<String>);

macro_rules! domain_error_handlers {
    ($($handler:ident),+ $(,)?) => {
        fn log_domain_error(error: &Errors) {
            $(
                crate::handlers::$handler::log_error(error);
            )+
        }

        fn map_domain_response(error: &Errors) -> Option<ErrorMapping> {
            [
                $(
                    crate::handlers::$handler::map_response as fn(&Errors) -> Option<ErrorMapping>,
                )+
            ]
            .into_iter()
            .find_map(|map_response| map_response(error))
        }
    };
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub status: u16,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

impl From<DbErr> for Errors {
    fn from(err: DbErr) -> Self {
        Errors::DatabaseError(err.to_string())
    }
}

impl From<TransactionError<DbErr>> for Errors {
    fn from(err: TransactionError<DbErr>) -> Self {
        Errors::TransactionError(err.to_string())
    }
}

#[derive(Debug)]
pub enum Errors {
    // Auth errors
    InvalidCredentials,

    // User errors
    UserInvalidPassword,
    UserPasswordNotSet,
    UserInvalidSession,
    UserNotVerified,
    UserNotFound,
    UserUnauthorized,
    UserBanned,
    UserPermissionInsufficient,
    AclDenied(String),
    UserHandleAlreadyExists,
    UserEmailAlreadyExists,
    UserNotBanned,
    UserAlreadyBanned,
    UserDoesNotHaveRole,
    UserAlreadyHasRole,
    CannotManageSelf,
    CannotManageHigherOrEqualRole,
    UserTokenExpired,
    UserNoRefreshToken,
    UserInvalidToken,

    // Session errors
    SessionInvalidUserId,
    SessionExpired,
    SessionNotFound,

    // Permission errors
    ForbiddenError(String),

    // Document
    DocumentNotFound,
    DocumentAlreadyExists,
    DocumentRevisionNotFound,
    DocumentNoChanges,
    DocumentEditRequestNotFound,
    DocumentDiscussionNotFound,
    DocumentRatingNotFound,

    // Discussion
    DiscussionMessageNotFound,
    DiscussionClosed,

    // Report
    ReportNotFound,
    ReportAlreadyExists,

    // OAuth
    OauthInvalidAuthUrl,
    OauthInvalidTokenUrl,
    OauthInvalidRedirectUrl,
    OauthTokenExchangeFailed,
    OauthUserInfoFetchFailed,
    OauthUserInfoParseFailed(String),
    OauthAccountAlreadyLinked,
    OauthConnectionNotFound,
    OauthCannotUnlinkLastConnection,
    OauthInvalidImageUrl,
    OauthInvalidState,
    OauthStateExpired,
    OauthHandleRequired,
    OauthEmailAlreadyExists,
    OauthEmailNotVerified,
    GoogleInvalidIdToken,
    GithubInvalidToken,
    GoogleJwksFetchFailed,
    GoogleJwksParseFailed,

    // Password errors
    PasswordRequiredForUpdate,
    PasswordIncorrect,
    PasswordCannotUpdateOauthOnly,
    PasswordNewPasswordMissing,
    PasswordAlreadySet,

    // Token errors
    TokenInvalidVerification,
    TokenExpiredVerification,
    TokenEmailMismatch,
    TokenInvalidReset,
    TokenExpiredReset,
    TokenInvalidEmailChange,

    // Email errors
    EmailAlreadyVerified,

    // File errors
    FileUploadError(String),
    FileNotFound,
    FileReadError(String),
    FileUnsupportedType(String),
    FileProcessingTimeout(String),
    FileProcessingUnavailable(String),

    // Worker Service errors
    WorkerServiceConnectionFailed,
    WorkerServiceResponseInvalid,
    VerificationEmailSendFailed,
    PasswordResetEmailSendFailed,

    // EventStream errors
    EventStreamPublishFailed,

    // General errors
    BadRequestError(String),
    ValidationError(String),
    FileTooLargeError(String),
    InvalidIpAddress,

    // System errors
    SysInternalError(String),
    DatabaseError(String),
    TransactionError(String),
    NotFound(String),
    HashingError(String),
    TokenCreationError(String),

    // Rate Limiting
    RateLimitExceeded,

    // Turnstile
    TurnstileTokenMissing,
    TurnstileVerificationFailed,
    TurnstileServiceError,

    // MeiliSearch
    MeiliSearchQueryFailed,

    // TOTP 2FA
    TotpAlreadyEnabled,
    TotpNotEnabled,
    TotpInvalidCode,
    TotpTempTokenInvalid,
    TotpTempTokenExpired,
    TotpBackupCodeExhausted,
    TotpSecretGenerationFailed,
    TotpQrGenerationFailed,
}

domain_error_handlers!(
    user_handler,
    oauth_handler,
    session_handler,
    password_handler,
    token_handler,
    totp_handler,
    email_handler,
    file_handler,
    worker_handler,
    eventstream_handler,
    rate_limit_handler,
    turnstile_handler,
    meilisearch_handler,
    system_handler,
    general_handler,
);

impl IntoResponse for Errors {
    fn into_response(self) -> Response {
        log_domain_error(&self);

        let (status, code, details) = map_domain_response(&self).unwrap_or_else(|| {
            error!("Unhandled error: {:?}", self);
            (StatusCode::INTERNAL_SERVER_ERROR, "UNKNOWN_ERROR", None)
        });

        // Only include details in dev mode
        let is_dev = ServerConfig::get().is_dev;

        // Construct error response
        let body = ErrorResponse {
            status: status.as_u16(),
            code: code.to_string(),
            details: if is_dev { details } else { None }, // Show details only in dev environment
        };

        (status, Json(body)).into_response()
    }
}

pub async fn handler_404<B>(req: axum::extract::Request<B>) -> impl IntoResponse {
    let path = req.uri().path();
    let method = req.method().to_string();

    Errors::NotFound(format!("Path {} with method {} not found", path, method))
}
