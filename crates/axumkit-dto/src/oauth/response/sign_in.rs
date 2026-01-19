use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth::response::create_login_response;
use crate::oauth::internal::SignInResult;
use axumkit_errors::errors::Errors;

/// 신규 사용자가 handle 없이 OAuth 로그인 시 반환되는 pending signup 응답
#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthPendingSignupResponse {
    /// Pending signup 완료를 위한 일회용 토큰
    pub pending_token: String,
    /// OAuth provider로부터 받은 이메일
    pub email: String,
    /// OAuth provider로부터 받은 표시 이름
    pub display_name: String,
}

impl IntoResponse for OAuthPendingSignupResponse {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// OAuth 로그인 결과를 HTTP 응답으로 변환
pub enum OAuthSignInResponse {
    /// 로그인 성공 - 204 No Content + Set-Cookie
    Success { session_id: String },
    /// Pending signup - 200 OK + JSON body
    PendingSignup(OAuthPendingSignupResponse),
}

impl OAuthSignInResponse {
    /// SignInResult를 OAuthSignInResponse로 변환
    pub fn from_result(result: SignInResult) -> Self {
        match result {
            SignInResult::Success(session_id) => OAuthSignInResponse::Success { session_id },
            SignInResult::PendingSignup {
                pending_token,
                email,
                display_name,
            } => OAuthSignInResponse::PendingSignup(OAuthPendingSignupResponse {
                pending_token,
                email,
                display_name,
            }),
        }
    }

    /// HTTP 응답으로 변환 (remember_me=true for OAuth, 30일 세션)
    pub fn into_response_result(self) -> Result<axum::response::Response, Errors> {
        match self {
            OAuthSignInResponse::Success { session_id } => {
                // OAuth 로그인은 항상 30일 유지 (remember_me=true)
                create_login_response(session_id, true)
            }
            OAuthSignInResponse::PendingSignup(response) => Ok(response.into_response()),
        }
    }
}
