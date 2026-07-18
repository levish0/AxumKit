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
    /// A sensitive action (e.g. account deletion) requires a re-authentication factor
    /// that the request did not supply.
    ReauthenticationRequired,
    UserInvalidSession,
    UserNotVerified,
    UserNotFound,
    UserUnauthorized,
    UserBanned,
    UserPermissionInsufficient,
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

    // ACL
    /// Denied by an ACL rule; the payload names the matched rule/condition.
    AclDenied(String),
    AclGroupNotFound,
    AclGroupAlreadyExists,
    AclGroupIsSystem,
    AclGroupMemberNotFound,
    AclGroupMemberAlreadyExists,
    AclInvalidRule(String),

    // Board
    BoardNotFound,
    BoardPostNotFound,
    BoardPostLocked,
    BoardPinSetMismatch,
    BoardCommentNotFound,

    // Post
    PostNotFound,

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
    GoogleOneTapNonceInvalid,
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
    TokenInvalidAccountDeletion,
    TokenInvalidDeviceVerify,

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
    /// A reindex of the same entity type is already in progress.
    ReindexAlreadyRunning,

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
    acl_handler,
    board_handler,
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
            error!(error = ?self, "Unhandled error");
            (StatusCode::INTERNAL_SERVER_ERROR, "UNKNOWN_ERROR", None)
        });

        // 5xx server errors hide details in production (no internal implementation leaks);
        // 4xx client errors always show details (the user needs to know the cause).
        let details = if status.is_server_error() && !ServerConfig::get().is_dev {
            None
        } else {
            details
        };

        let body = ErrorResponse {
            status: status.as_u16(),
            code: code.to_string(),
            details,
        };

        (status, Json(body)).into_response()
    }
}

pub async fn handler_404<B>(req: axum::extract::Request<B>) -> impl IntoResponse {
    let path = req.uri().path();
    let method = req.method().to_string();

    Errors::NotFound(format!("Path {} with method {} not found", path, method))
}
