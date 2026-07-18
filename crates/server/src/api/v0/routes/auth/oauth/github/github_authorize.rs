use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::github::service_generate_github_oauth_url;
use crate::state::AppState;
use axum::Extension;
use axum::extract::State;
use dto::oauth::request::{OAuthAuthorizeFlow, OAuthAuthorizeQuery};
use dto::oauth::response::OAuthUrlResponse;
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

/// Generates a GitHub OAuth authorization URL.
#[utoipa::path(
    get,
    path = "/v0/auth/oauth/github/authorize",
    summary = "Create a GitHub OAuth authorization URL",
    description = "Generates a GitHub authorization URL with PKCE and stores a single-use state record in Redis. The state is bound to the current anonymous browser context and to the requested flow, which defaults to login.",
    params(OAuthAuthorizeQuery),
    responses(
        (status = 200, description = "Authorization URL generated successfully", body = OAuthUrlResponse),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 500, description = "Unexpected Redis or OAuth URL generation error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_github_authorize(
    State(state): State<AppState>,
    Extension(anonymous): Extension<AnonymousUserContext>,
    ValidatedQuery(query): ValidatedQuery<OAuthAuthorizeQuery>,
) -> Result<OAuthUrlResponse, Errors> {
    let flow = query.flow.unwrap_or(OAuthAuthorizeFlow::Login);

    service_generate_github_oauth_url(&state.redis_session, &anonymous.anonymous_user_id, flow)
        .await
}
