use crate::config::db_config::DbConfig;
use crate::errors::protocol::email::EMAIL_ALREADY_VERIFIED;
use crate::errors::protocol::file::{FILE_NOT_FOUND, FILE_READ_ERROR, FILE_UPLOAD_ERROR};

use crate::errors::protocol::general::{BAD_REQUEST, VALIDATION_ERROR};
use crate::errors::protocol::password::{
    PASSWORD_ALREADY_SET, PASSWORD_CANNOT_UPDATE_OAUTH_ONLY, PASSWORD_INCORRECT,
    PASSWORD_NEW_PASSWORD_MISSING, PASSWORD_REQUIRED_FOR_UPDATE,
};
use crate::errors::protocol::session::{
    SESSION_EXPIRED, SESSION_INVALID_USER_ID, SESSION_NOT_FOUND,
};
use crate::errors::protocol::system::{
    SYS_DATABASE_ERROR, SYS_HASHING_ERROR, SYS_INTERNAL_ERROR, SYS_NOT_FOUND,
    SYS_TOKEN_CREATION_ERROR, SYS_TRANSACTION_ERROR,
};
use crate::errors::protocol::token::{
    TOKEN_EMAIL_MISMATCH, TOKEN_EXPIRED_RESET, TOKEN_EXPIRED_VERIFICATION, TOKEN_INVALID_RESET,
    TOKEN_INVALID_VERIFICATION,
};
use crate::errors::protocol::user::{
    USER_BANNED, USER_EMAIL_ALREADY_EXISTS, USER_HANDLE_ALREADY_EXISTS, USER_INVALID_PASSWORD,
    USER_INVALID_SESSION, USER_INVALID_TOKEN, USER_NO_REFRESH_TOKEN, USER_NOT_FOUND,
    USER_NOT_VERIFIED, USER_PASSWORD_NOT_SET, USER_PERMISSION_INSUFFICIENT, USER_TOKEN_EXPIRED,
    USER_UNAUTHORIZED,
};
use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sea_orm::{DbErr, TransactionError};
use serde::Serialize;
use tracing::{debug, error, warn};
use utoipa::ToSchema;
// 이 모듈은 애플리케이션의 오류 처리 시스템을 구현합니다.
// 주요 기능:
// 1. 다양한 오류 유형 정의 (사용자, 문서, 권한, 시스템 등)
// 2. 오류를 HTTP 응답으로 변환하는 메커니즘
// 3. 데이터베이스 오류를 애플리케이션 오류로 변환하는 기능

// 표준화된 Result 타입 정의
pub type ServiceResult<T> = Result<T, Errors>;
pub type ApiResult<T> = Result<T, Errors>;

// ErrorResponse 구조체: API 응답에서 오류를 표현하기 위한 구조체
// status: HTTP 상태 코드
// code: 오류 코드 문자열
// details: 개발 환경에서만 표시되는 상세 오류 메시지 (선택적)
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
    fn from(err: sea_orm::DbErr) -> Self {
        Errors::DatabaseError(err.to_string())
    }
}

// 트랜잭션 오류를 애플리케이션 오류로 변환
impl From<TransactionError<DbErr>> for Errors {
    fn from(err: TransactionError<DbErr>) -> Self {
        Errors::TransactionError(err.to_string())
    }
}

// 애플리케이션에서 발생할 수 있는 모든 오류 유형을 정의하는 열거형
// 카테고리별로 구분되어 있으며, 일부 오류는 추가 정보를 포함할 수 있음
#[derive(Debug)]
pub enum Errors {
    // 사용자 관련 오류
    UserInvalidPassword, // 잘못된 비밀번호
    UserPasswordNotSet,  // 비밀번호가 설정되지 않음 (OAuth 전용 사용자)
    UserInvalidSession,  // 잘못된 세션 데이터
    UserNotVerified,
    UserNotFound,               // 사용자를 찾을 수 없음
    UserUnauthorized,           // 인증되지 않은 사용자
    UserBanned,                 // 사용자가 밴됨
    UserPermissionInsufficient, // 권한 부족
    UserHandleAlreadyExists,    // 핸들이 이미 존재함
    UserEmailAlreadyExists,     // 이메일이 이미 존재함
    UserTokenExpired,           // 만료된 토큰
    UserNoRefreshToken,
    UserInvalidToken, // 유효하지 않은 토큰

    // 세션 관련 오류
    SessionInvalidUserId, // 세션의 user_id가 유효하지 않은 UUID 형식
    SessionExpired,       // 만료된 세션
    SessionNotFound,      // 세션을 찾을 수 없음

    // 권한 관련 오류
    ForbiddenError(String), // 403 Forbidden - 접근 권한 없음

    // Password related errors
    PasswordRequiredForUpdate,
    PasswordIncorrect,
    PasswordCannotUpdateOauthOnly,
    PasswordNewPasswordMissing,
    PasswordAlreadySet,

    // Token related errors
    TokenInvalidVerification,
    TokenExpiredVerification,
    TokenEmailMismatch,
    TokenInvalidReset,
    TokenExpiredReset,

    // Email errors
    EmailAlreadyVerified,

    // File related errors
    FileUploadError(String),
    FileNotFound,
    FileReadError(String),

    // 일반 오류
    BadRequestError(String),   // 잘못된 요청 (추가 정보 포함)
    ValidationError(String),   // 유효성 검사 오류 (추가 정보 포함)
    FileTooLargeError(String), // 파일 크기 초과 오류

    // 시스템 오류
    SysInternalError(String),
    DatabaseError(String),      // 데이터베이스 오류 (추가 정보 포함)
    TransactionError(String),   // 트랜잭션 오류 (추가 정보 포함)
    NotFound(String),           // 리소스를 찾을 수 없음 (추가 정보 포함)
    HashingError(String),       // 해싱 오류 (추가 정보 포함)
    TokenCreationError(String), // 토큰 생성 오류 (추가 정보 포함)
}

// IntoResponse 트레이트 구현: Errors를 HTTP 응답으로 변환
// 각 오류 유형에 적절한 HTTP 상태 코드와 오류 코드를 매핑
// 중앙집중식 로깅도 여기서 처리
impl IntoResponse for Errors {
    fn into_response(self) -> Response {
        // 에러 레벨에 따른 중앙집중식 로깅
        match &self {
            // 시스템 심각도 에러 - error! 레벨
            Errors::SysInternalError(_)
            | Errors::DatabaseError(_)
            | Errors::TransactionError(_)
            | Errors::HashingError(_)
            | Errors::TokenCreationError(_) => {
                error!("System error occurred: {:?}", self);
            }

            // 리소스 찾을 수 없음 - warn! 레벨
            Errors::UserNotFound | Errors::NotFound(_) => {
                warn!("Resource not found: {:?}", self);
            }

            // 비즈니스 로직 에러 - debug! 레벨 (클라이언트 실수)
            Errors::UserInvalidPassword
            | Errors::UserPasswordNotSet
            | Errors::UserInvalidSession
            | Errors::UserNotVerified
            | Errors::UserUnauthorized
            | Errors::UserBanned
            | Errors::UserPermissionInsufficient
            | Errors::UserHandleAlreadyExists
            | Errors::UserEmailAlreadyExists
            | Errors::UserTokenExpired
            | Errors::UserNoRefreshToken
            | Errors::UserInvalidToken
            | Errors::SessionInvalidUserId
            | Errors::SessionExpired
            | Errors::SessionNotFound
            | Errors::ForbiddenError(_)
            | Errors::PasswordRequiredForUpdate
            | Errors::PasswordIncorrect
            | Errors::PasswordCannotUpdateOauthOnly
            | Errors::PasswordNewPasswordMissing
            | Errors::PasswordAlreadySet
            | Errors::TokenInvalidVerification
            | Errors::TokenExpiredVerification
            | Errors::TokenEmailMismatch
            | Errors::TokenInvalidReset
            | Errors::TokenExpiredReset
            | Errors::EmailAlreadyVerified
            | Errors::BadRequestError(_)
            | Errors::ValidationError(_)
            | Errors::FileTooLargeError(_) => {
                debug!("Client error: {:?}", self);
            }

            // 파일 관련 에러 - warn! 레벨
            Errors::FileUploadError(_) | Errors::FileNotFound | Errors::FileReadError(_) => {
                warn!("File/processing error: {:?}", self);
            }
        }

        // 오류 유형에 따라 상태 코드, 오류 코드, 상세 정보를 결정
        let (status, code, details) = match self {
            // 사용자 관련 오류 - 주로 401 Unauthorized 또는 404 Not Found
            Errors::UserInvalidPassword => (StatusCode::UNAUTHORIZED, USER_INVALID_PASSWORD, None),
            Errors::UserPasswordNotSet => (StatusCode::UNAUTHORIZED, USER_PASSWORD_NOT_SET, None),
            Errors::UserInvalidSession => (StatusCode::UNAUTHORIZED, USER_INVALID_SESSION, None),
            Errors::UserNotVerified => (StatusCode::UNAUTHORIZED, USER_NOT_VERIFIED, None),
            Errors::UserNotFound => (StatusCode::NOT_FOUND, USER_NOT_FOUND, None),
            Errors::UserUnauthorized => (StatusCode::UNAUTHORIZED, USER_UNAUTHORIZED, None),
            Errors::UserBanned => (StatusCode::FORBIDDEN, USER_BANNED, None),
            Errors::UserPermissionInsufficient => {
                (StatusCode::FORBIDDEN, USER_PERMISSION_INSUFFICIENT, None)
            }
            Errors::UserHandleAlreadyExists => {
                (StatusCode::CONFLICT, USER_HANDLE_ALREADY_EXISTS, None)
            }
            Errors::UserEmailAlreadyExists => {
                (StatusCode::CONFLICT, USER_EMAIL_ALREADY_EXISTS, None)
            }
            Errors::UserTokenExpired => (StatusCode::UNAUTHORIZED, USER_TOKEN_EXPIRED, None),
            Errors::UserNoRefreshToken => (StatusCode::UNAUTHORIZED, USER_NO_REFRESH_TOKEN, None),
            Errors::UserInvalidToken => (StatusCode::UNAUTHORIZED, USER_INVALID_TOKEN, None),

            // 세션 관련 오류
            Errors::SessionInvalidUserId => {
                (StatusCode::UNAUTHORIZED, SESSION_INVALID_USER_ID, None)
            }
            Errors::SessionExpired => (StatusCode::UNAUTHORIZED, SESSION_EXPIRED, None),
            Errors::SessionNotFound => (StatusCode::UNAUTHORIZED, SESSION_NOT_FOUND, None),

            Errors::ForbiddenError(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", Some(msg.clone())),

            // Password errors
            Errors::PasswordRequiredForUpdate => {
                (StatusCode::BAD_REQUEST, PASSWORD_REQUIRED_FOR_UPDATE, None)
            }
            Errors::PasswordIncorrect => (StatusCode::BAD_REQUEST, PASSWORD_INCORRECT, None),
            Errors::PasswordCannotUpdateOauthOnly => (
                StatusCode::BAD_REQUEST,
                PASSWORD_CANNOT_UPDATE_OAUTH_ONLY,
                None,
            ),
            Errors::PasswordNewPasswordMissing => {
                (StatusCode::BAD_REQUEST, PASSWORD_NEW_PASSWORD_MISSING, None)
            }
            Errors::PasswordAlreadySet => (StatusCode::BAD_REQUEST, PASSWORD_ALREADY_SET, None),

            // Token errors
            Errors::TokenInvalidVerification => {
                (StatusCode::BAD_REQUEST, TOKEN_INVALID_VERIFICATION, None)
            }
            Errors::TokenExpiredVerification => {
                (StatusCode::BAD_REQUEST, TOKEN_EXPIRED_VERIFICATION, None)
            }
            Errors::TokenEmailMismatch => (StatusCode::BAD_REQUEST, TOKEN_EMAIL_MISMATCH, None),
            Errors::TokenInvalidReset => (StatusCode::BAD_REQUEST, TOKEN_INVALID_RESET, None),
            Errors::TokenExpiredReset => (StatusCode::BAD_REQUEST, TOKEN_EXPIRED_RESET, None),

            // Email errors
            Errors::EmailAlreadyVerified => (StatusCode::BAD_REQUEST, EMAIL_ALREADY_VERIFIED, None),

            // File errors
            Errors::FileUploadError(msg) => (StatusCode::BAD_REQUEST, FILE_UPLOAD_ERROR, Some(msg)),
            Errors::FileNotFound => (StatusCode::BAD_REQUEST, FILE_NOT_FOUND, None),
            Errors::FileReadError(msg) => (StatusCode::BAD_REQUEST, FILE_READ_ERROR, Some(msg)),

            // 일반 오류 - 400 Bad Request
            Errors::BadRequestError(msg) => (StatusCode::BAD_REQUEST, BAD_REQUEST, Some(msg)),
            Errors::ValidationError(msg) => (StatusCode::BAD_REQUEST, VALIDATION_ERROR, Some(msg)),
            Errors::FileTooLargeError(msg) => {
                (StatusCode::PAYLOAD_TOO_LARGE, "FILE_TOO_LARGE", Some(msg))
            }

            // 시스템 오류 - 주로 500 Internal Server Error
            Errors::SysInternalError(msg) => {
                (StatusCode::BAD_REQUEST, SYS_INTERNAL_ERROR, Some(msg))
            }
            Errors::TransactionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                SYS_TRANSACTION_ERROR,
                Some(msg),
            ),
            Errors::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                SYS_DATABASE_ERROR,
                Some(msg),
            ),
            Errors::NotFound(msg) => (StatusCode::NOT_FOUND, SYS_NOT_FOUND, Some(msg)),
            Errors::HashingError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                SYS_HASHING_ERROR,
                Some(msg),
            ),
            Errors::TokenCreationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                SYS_TOKEN_CREATION_ERROR,
                Some(msg),
            ),
        };

        // 개발 환경에서만 상세 오류 정보 포함
        let is_dev = DbConfig::get().is_dev;

        // 오류 응답 구성
        let body = ErrorResponse {
            status: status.as_u16(),
            code: code.to_string(),
            details: if is_dev { details } else { None }, // 개발 환경에서만 상세 정보 표시
        };

        // HTTP 응답으로 변환하여 반환
        (status, Json(body)).into_response()
    }
}

// 404 오류 처리 핸들러 함수
// 요청된 경로가 존재하지 않을 때 호출되는 전역 핸들러
pub async fn handler_404<B>(req: Request<B>) -> impl IntoResponse {
    // 요청 경로와 HTTP 메서드 추출
    let path = req.uri().path();
    let method = req.method().to_string();

    // NotFound 오류 반환 - 로깅은 IntoResponse에서 중앙집중화하여 처리
    Errors::NotFound(format!("Path {} with method {} not found", path, method))
}
