use crate::dto::oauth::response::oauth_url::OAuthUrlResponse;
use crate::errors::errors::Errors;
use crate::service::oauth::generate_google_oauth_url::service_generate_google_oauth_url;
use crate::state::AppState;
use axum::extract::State;

/// Google OAuth 인증 URL을 생성합니다.
#[utoipa::path(
    get,
    path = "/v0/auth/google/authorize",
    responses(
        (status = 200, description = "OAuth URL generated", body = OAuthUrlResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn auth_google_authorize(
    State(state): State<AppState>,
) -> Result<OAuthUrlResponse, Errors> {
    service_generate_google_oauth_url(&state.redis_client).await
}
