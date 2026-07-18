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
use crate::permission::PermissionService;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Request};
use axum::middleware::Next;
use axum::response::Response;
use entity::common::Role;
use errors::errors::Errors;
use tower_cookies::Cookies;

/// Resolve the caller's session and demand `role`, or reject with
/// `UserUnauthorized`. Uses the role-only context path — group permissions are
/// irrelevant to a role gate and are not loaded.
async fn gate(
    state: &AppState,
    cookies: &Cookies,
    headers: &HeaderMap,
    role: Role,
) -> Result<(), Errors> {
    let session = resolve_session_from_request(cookies, headers, state)
        .await?
        .ok_or(Errors::UserUnauthorized)?;
    PermissionService::require_role(&state.db, Some(&session), role).await
}

/// Router-boundary gate: require the `Admin` role.
pub async fn require_admin(
    State(state): State<AppState>,
    cookies: Cookies,
    headers: HeaderMap,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Errors> {
    gate(&state, &cookies, &headers, Role::Admin).await?;
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
    gate(&state, &cookies, &headers, Role::Mod).await?;
    Ok(next.run(req).await)
}
