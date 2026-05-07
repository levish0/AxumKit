use crate::extractors::RequiredSession;
use crate::service::auth::totp::service_totp_enable;
use crate::state::AppState;
use axum::extract::State;
use dto::auth::request::TotpEnableRequest;
use dto::auth::response::TotpEnableResponse;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/totp/enable",
    request_body = TotpEnableRequest,
    responses(
        (status = 200, description = "TOTP enabled", body = TotpEnableResponse),
        (status = 400, description = "Invalid TOTP code"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "TOTP already enabled"),
        (status = 500, description = "Internal Server Error")
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_enable(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<TotpEnableRequest>,
) -> Result<TotpEnableResponse, Errors> {
    service_totp_enable(&state.db, session.user_id, &payload.code).await
}
