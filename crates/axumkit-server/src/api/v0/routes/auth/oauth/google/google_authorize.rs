use crate::service::oauth::google::service_generate_google_oauth_url;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::oauth::response::OAuthUrlResponse;
use axumkit_errors::errors::Errors;

/// Google OAuth 인증 URL을 생성합니다.
#[utoipa::path(
    get,
    path = "/v0/auth/oauth/google/authorize",
    responses(
        (status = 200, description = "OAuth URL generated", body = OAuthUrlResponse),
        (status = 500, description = "Internal Server Error - Redis or OAuth URL generation error")
    ),
    tag = "Auth"
)]
pub async fn auth_google_authorize(
    State(state): State<AppState>,
) -> Result<OAuthUrlResponse, Errors> {
    service_generate_google_oauth_url(&state.redis_session).await
}
