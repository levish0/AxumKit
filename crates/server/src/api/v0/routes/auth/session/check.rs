use crate::extractors::session::resolve_session_from_request;
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use dto::auth::response::{AuthCheckIdentity, create_auth_check_response};
use tower_cookies::Cookies;

#[utoipa::path(
    get,
    path = "/v0/auth/check",
    summary = "Resolve the session identity for the gateway (forward-auth)",
    description = "Internal endpoint called by the API gateway (APISIX) on every request via \
        forward-auth. It validates the session credential against the session store and the \
        database, then always responds 200 with `X-Auth-Status: authenticated|anonymous` plus the \
        user/session/management identifiers on success. The gateway uses these only to bucket \
        rate limits (per-user vs. per-IP); it never blocks here. The backend does not consume \
        these headers for authorization — each route re-validates the session credential itself.",
    responses(
        (status = 200, description = "Identity resolved; see X-Auth-* response headers")
    ),
    tag = "Auth"
)]
pub async fn auth_check(
    State(state): State<AppState>,
    cookies: Cookies,
    headers: HeaderMap,
) -> Response {
    // Non-blocking identity resolver: any validation failure simply resolves to anonymous.
    // Reads the session cookie (browser) or `Authorization: Bearer` (native app) so the gateway
    // buckets authenticated app traffic per-user, not per-IP.
    let identity = resolve_session_from_request(&cookies, &headers, &state)
        .await
        .ok()
        .flatten()
        .map(|context| AuthCheckIdentity {
            user_id: context.user_id,
            session_id: context.session_id,
            management_id: context.management_id,
        });

    create_auth_check_response(identity)
}
