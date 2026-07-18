use crate::service::auth::device::confirm_device_verification;
use crate::state::AppState;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use dto::auth::request::VerifyDeviceRequest;
use dto::auth::response::{AppDeviceVerifyResponse, build_device_cookie, create_login_response};
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

#[utoipa::path(
    post,
    path = "/v0/app/auth/device/verify",
    summary = "Confirm a new-device sign-in (native-app client)",
    description = "Native-app variant of POST /v0/auth/device/verify. Consumes the single-use token \
        emailed after an app login was held for new-device verification, trusts the device, and \
        returns both the session token and a device-recognition token in the response body (app \
        clients have no cookie jar). The app replays the session token as `Authorization: Bearer` \
        and stores the device token to send in the `X-Device-Token` header on future logins, so \
        this device is not challenged again. The emailed token is the proof, so no session is \
        required.",
    request_body = VerifyDeviceRequest,
    responses(
        (status = 200, description = "Device verified; session and device tokens are returned in the body", body = AppDeviceVerifyResponse),
        (status = 400, description = "Invalid or expired verification token", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or session store error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_verify_device_app(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<VerifyDeviceRequest>,
) -> Result<Response, Errors> {
    let result =
        confirm_device_verification(&state.db, &state.redis_session, &payload.token).await?;

    // App holds both tokens itself → return them in the body (no cookies). `remember_me` is
    // irrelevant for apps: there is no cookie to persist, only the server-side session lifetime.
    Ok(AppDeviceVerifyResponse::new(result.session_token, result.device_token).into_response())
}
