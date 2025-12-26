use crate::dto::auth::LoginRequest;
use crate::dto::oauth::request::github::GithubLoginRequest;
use crate::dto::oauth::request::google::GoogleLoginRequest;
use crate::dto::oauth::response::oauth_url::OAuthUrlResponse;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::login::auth_login,
        super::logout::auth_logout,
        super::google_authorize::auth_google_authorize,
        super::google_login::auth_google_login,
        super::github_authorize::auth_github_authorize,
        super::github_login::auth_github_login,
    ),
    components(
        schemas(
            LoginRequest,
            OAuthUrlResponse,
            GoogleLoginRequest,
            GithubLoginRequest,
        )
    ),
    tags(
        (name = "Auth", description = "Authentication endpoints")
    )
)]
pub struct AuthApiDoc;
