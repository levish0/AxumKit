use super::responses::redirect_response;
use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, read_cookie},
    middleware::require_browser_origin,
    v1::identity::contracts::OidcRedirectOutput,
};
use axum::{extract::State, http::HeaderMap, response::Response};

#[utoipa::path(post, path="/api/v1/auth/oidc/{provider}", tag="OIDC", params(("provider"=String,Path)), responses((status=200,body=OidcRedirectOutput)))]
pub async fn begin_login(
    State(state): State<HttpState>,
    ApiPath(provider): ApiPath<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    require_browser_origin(&state, &headers)?;
    let binding = read_cookie(&headers, state.settings.oauth_cookie_name())?;
    let redirect = state
        .app
        .oidc
        .begin_authorization(&provider, None, binding.as_deref())
        .await?;
    Ok(redirect_response(&state, redirect))
}
