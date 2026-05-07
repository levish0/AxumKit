use crate::service::auth::signup::service_signup;
use crate::state::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use dto::user::{CreateUserRequest, CreateUserResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/signup",
    summary = "Start an email and password signup",
    description = "Validates the requested email, handle, and password, stores a pending signup in Redis, and queues a verification email. The user account is created only after the token is submitted to POST /v0/auth/verify-email.",
    request_body = CreateUserRequest,
    responses(
        (status = 202, description = "Verification email queued and pending signup stored", body = CreateUserResponse),
        (status = 400, description = "Malformed JSON payload or validation error", body = ErrorResponse),
        (status = 409, description = "The email or handle is already in use or reserved by another pending signup", body = ErrorResponse),
        (status = 500, description = "Unexpected database or Redis error", body = ErrorResponse),
        (status = 502, description = "Worker service rejected the verification email job or returned an invalid response", body = ErrorResponse),
        (status = 503, description = "Worker service could not be reached", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn auth_signup(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserRequest>,
) -> Result<impl IntoResponse, Errors> {
    let response = service_signup(&state.db, &state.redis_session, &state.worker, payload).await?;

    Ok((StatusCode::ACCEPTED, Json(response)))
}
