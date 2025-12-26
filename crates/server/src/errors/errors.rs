use crate::config::server_config::ServerConfig;
use crate::errors::handlers::{
    email_handler, file_handler, general_handler, oauth_handler, password_handler,
    rate_limit_handler, session_handler, system_handler, token_handler, user_handler,
};
use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sea_orm::{DbErr, TransactionError};
use serde::Serialize;
use tracing::error;
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

    // oauth
    OauthInvalidAuthUrl,
    OauthInvalidTokenUrl,
    OauthInvalidRedirectUrl,
    OauthTokenExchangeFailed,
    OauthUserInfoFetchFailed,
    OauthUserInfoParseFailed(String), // OAuth 사용자 정보 파싱 실패 (응답 내용 포함)
    OauthAccountAlreadyLinked,
    OauthConnectionNotFound,
    OauthCannotUnlinkLastConnection,
    OauthInvalidImageUrl,
    OauthInvalidState,
    OauthStateExpired,
    OauthHandleRequired,

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
    InvalidIpAddress,          // 유효하지 않은 IP 주소

    // 시스템 오류
    SysInternalError(String),
    DatabaseError(String),      // 데이터베이스 오류 (추가 정보 포함)
    TransactionError(String),   // 트랜잭션 오류 (추가 정보 포함)
    NotFound(String),           // 리소스를 찾을 수 없음 (추가 정보 포함)
    HashingError(String),       // 해싱 오류 (추가 정보 포함)
    TokenCreationError(String), // 토큰 생성 오류 (추가 정보 포함)

    // Rate Limiting
    RateLimitExceeded, // 요청 제한 초과
}

// IntoResponse 트레이트 구현: Errors를 HTTP 응답으로 변환
// 각 오류 유형에 적절한 HTTP 상태 코드와 오류 코드를 매핑
// 중앙집중식 로깅도 여기서 처리
impl IntoResponse for Errors {
    fn into_response(self) -> Response {
        // 도메인별 handler를 통한 중앙집중식 로깅
        user_handler::log_error(&self);
        oauth_handler::log_error(&self);
        session_handler::log_error(&self);
        password_handler::log_error(&self);
        token_handler::log_error(&self);
        email_handler::log_error(&self);
        rate_limit_handler::log_error(&self);
        file_handler::log_error(&self);
        system_handler::log_error(&self);
        general_handler::log_error(&self);

        // 도메인별 handler를 통한 HTTP 응답 매핑
        let (status, code, details) = user_handler::map_response(&self)
            .or_else(|| oauth_handler::map_response(&self))
            .or_else(|| session_handler::map_response(&self))
            .or_else(|| password_handler::map_response(&self))
            .or_else(|| token_handler::map_response(&self))
            .or_else(|| email_handler::map_response(&self))
            .or_else(|| rate_limit_handler::map_response(&self))
            .or_else(|| file_handler::map_response(&self))
            .or_else(|| system_handler::map_response(&self))
            .or_else(|| general_handler::map_response(&self))
            .unwrap_or_else(|| {
                // Fallback: 처리되지 않은 에러
                error!("Unhandled error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "UNKNOWN_ERROR", None)
            });

        // 개발 환경에서만 상세 오류 정보 포함
        let is_dev = ServerConfig::get().is_dev;

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
