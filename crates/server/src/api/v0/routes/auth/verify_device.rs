use crate::service::auth::device::confirm_device_verification;
use crate::state::AppState;
use axum::extract::State;
use axum::response::Response;
use dto::auth::request::VerifyDeviceRequest;
use dto::auth::response::{build_device_cookie, create_login_response};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use tower_cookies::Cookies;

#[utoipa::path(
    post,
    path = "/v0/auth/device/verify",
    summary = "Confirm a new-device sign-in",
    description = "Completes a login that was held for new-device verification (OWASP ASVS 6.3.5). \
        Consumes the single-use token delivered to the account email, trusts the device (sets the \
        long-lived device cookie), and issues the session cookie. The emailed token is the proof, \
        so no session is required.",
    request_body = VerifyDeviceRequest,
    responses(
        (status = 204, description = "Device verified; session and device cookies were issued"),
        (status = 400, description = "Invalid or expired verification token", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or session store error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_verify_device(
    State(state): State<AppState>,
    cookies: Cookies,
    ValidatedJson(payload): ValidatedJson<VerifyDeviceRequest>,
) -> Result<Response, Errors> {
    let result =
        confirm_device_verification(&state.db, &state.redis_session, &payload.token).await?;

    // Trust this browser going forward: set the long-lived device cookie (the CookieManager layer
    // writes it as a Set-Cookie alongside the session cookie).
    cookies.add(build_device_cookie(result.device_token));

    create_login_response(result.session_token, result.remember_me)
}
