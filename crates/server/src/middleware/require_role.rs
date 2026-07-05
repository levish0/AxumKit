//! Router-boundary authorization gates.
//!
//! Authentication is an extractor ([`crate::extractors::session`]); coarse authorization
//! (role tiers) belongs at the router boundary so that "which routes require which privilege" is a
//! single greppable property of the route table — a handler can no longer silently expose an
//! endpoint by forgetting an in-service check. Fine-grained, resource-scoped rules stay in
//! [`crate::permission::PermissionService`].
//!
//! Apply with `route_layer(from_fn_with_state(state, require_admin))` on the sub-router that should
//! be gated.

use crate::extractors::session::resolve_session_from_request;
use crate::permission::{PermissionService, UserContext};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Request};
use axum::middleware::Next;
use axum::response::Response;
use entity::common::Role;
use errors::errors::Errors;
use tower_cookies::Cookies;

/// Resolve the caller's authenticated permission context, or reject with `UserUnauthorized`.
async fn authorized_context(
    state: &AppState,
    cookies: &Cookies,
    headers: &HeaderMap,
) -> Result<UserContext, Errors> {
    let session = resolve_session_from_request(cookies, headers, state)
        .await?
        .ok_or(Errors::UserUnauthorized)?;
    PermissionService::get_context(&state.db, Some(&session)).await
}

/// Router-boundary gate: require the `Admin` role.
pub async fn require_admin(
    State(state): State<AppState>,
    cookies: Cookies,
    headers: HeaderMap,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Errors> {
    authorized_context(&state, &cookies, &headers)
        .await?
        .require_role(Role::Admin)?;
    Ok(next.run(req).await)
}

/// Router-boundary gate: require at least the `Mod` role (any moderator, or an `Admin` — the
/// `Admin` role satisfies every `require_role` check).
pub async fn require_mod(
    State(state): State<AppState>,
    cookies: Cookies,
    headers: HeaderMap,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Errors> {
    authorized_context(&state, &cookies, &headers)
        .await?
        .require_role(Role::Mod)?;
    Ok(next.run(req).await)
}
