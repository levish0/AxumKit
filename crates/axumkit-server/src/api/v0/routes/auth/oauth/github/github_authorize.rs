use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::github::service_generate_github_oauth_url;
use crate::state::AppState;
use axum::Extension;
use axum::extract::State;
use axumkit_dto::oauth::request::{OAuthAuthorizeFlow, OAuthAuthorizeQuery};
use axumkit_dto::oauth::response::OAuthUrlResponse;
use axumkit_dto::validator::query_validator::ValidatedQuery;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/auth/oauth/github/authorize",
    params(OAuthAuthorizeQuery),
    responses(
        (status = 200, description = "OAuth URL generated", body = OAuthUrlResponse),
        (status = 500, description = "Internal Server Error - Redis or OAuth URL generation error")
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
