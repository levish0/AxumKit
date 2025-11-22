use super::github_authorize::auth_github_authorize;
use super::github_login::auth_github_login;
use super::google_authorize::auth_google_authorize;
use super::google_login::auth_google_login;
use super::login::auth_login;
use super::logout::auth_logout;
use crate::middleware::auth::session_auth;
use crate::middleware::rate_limit::{RateLimitConfig, rate_limit};
use crate::state::AppState;
use axum::{Extension, Router, middleware, routing::get, routing::post};
use tower::ServiceBuilder;

const OAUTH_AUTHORIZE_LIMIT: RateLimitConfig = RateLimitConfig {
    route_name: "auth.oauth_authorize",
    max_requests: 10,
    window_secs: 60,
};

const OAUTH_LOGIN_LIMIT: RateLimitConfig = RateLimitConfig {
    route_name: "auth.oauth_login",
    max_requests: 10,
    window_secs: 60,
};

const EMAIL_LOGIN_LIMIT: RateLimitConfig = RateLimitConfig {
    route_name: "auth.email_login",
    max_requests: 10,
    window_secs: 60,
};

pub fn auth_routes(state: AppState) -> Router<AppState> {
    // Protected routes (require authentication)
    let protected_routes = Router::new().route("/auth/logout", post(auth_logout));

    // OAuth authorize routes (URL generation) - 10 req/min
    let oauth_authorize_routes = Router::new()
        .route("/auth/google/authorize", get(auth_google_authorize))
        .route("/auth/github/authorize", get(auth_github_authorize))
        .route_layer(
            ServiceBuilder::new()
                .layer(Extension(OAUTH_AUTHORIZE_LIMIT))
                .layer(middleware::from_fn_with_state(state.clone(), rate_limit)),
        );

    // OAuth login routes (code exchange) - 5 req/min
    let oauth_login_routes = Router::new()
        .route("/auth/google", post(auth_google_login))
        .route("/auth/github", post(auth_github_login))
        .route_layer(
            ServiceBuilder::new()
                .layer(Extension(OAUTH_LOGIN_LIMIT))
                .layer(middleware::from_fn_with_state(state.clone(), rate_limit)),
        );

    // Email/password login route - 5 req/min
    let email_login_routes = Router::new()
        .route("/auth/login", post(auth_login))
        .route_layer(
            ServiceBuilder::new()
                .layer(Extension(EMAIL_LOGIN_LIMIT))
                .layer(middleware::from_fn_with_state(state.clone(), rate_limit)),
        );

    // Merge all routes
    protected_routes
        .merge(oauth_authorize_routes)
        .merge(oauth_login_routes)
        .merge(email_login_routes)
}
