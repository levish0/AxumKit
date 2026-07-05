use crate::service::user::account::delete_my_account::service_confirm_account_deletion;
use crate::state::AppState;
use axum::extract::State;
use axum::response::Response;
use dto::auth::response::create_logout_response;
use dto::user::ConfirmAccountDeletionRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/user/me/deletion/confirm",
    summary = "Confirm account deletion",
    description = "Completes a deferred account deletion for OAuth-only accounts using the \
        single-use token delivered to the account's email address. The emailed token is the \
        re-authentication proof, so no session is required.",
    request_body = ConfirmAccountDeletionRequest,
    responses(
        (status = 204, description = "Account deleted successfully"),
        (status = 400, description = "Bad Request - Invalid or expired token", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or session error", body = ErrorResponse)
    ),
    tag = "User",
)]
pub async fn confirm_account_deletion(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ConfirmAccountDeletionRequest>,
) -> Result<Response, Errors> {
    service_confirm_account_deletion(
        &state.db,
        &state.redis_session,
        &state.worker,
        &payload.token,
    )
    .await?;
    create_logout_response()
}
