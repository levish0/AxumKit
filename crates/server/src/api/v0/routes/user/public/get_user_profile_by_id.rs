use crate::service::user::public::get_user_profile_by_id::service_get_user_profile_by_id;
use crate::state::AppState;
use axum::extract::State;
use dto::user::{GetUserProfileByIdRequest, PublicUserProfile};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/users/profile/id",
    summary = "Get a user profile by ID",
    description = "Returns the public profile for the requested user ID.",
    params(GetUserProfileByIdRequest),
    responses(
        (status = 200, description = "User profile retrieved successfully", body = PublicUserProfile),
        (status = 400, description = "Bad request - Invalid query parameters", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "User"
)]
pub async fn get_user_profile_by_id(
    State(state): State<AppState>,
    ValidatedQuery(payload): ValidatedQuery<GetUserProfileByIdRequest>,
) -> Result<PublicUserProfile, Errors> {
    let profile = service_get_user_profile_by_id(&state.db, payload.user_id).await?;
    Ok(profile)
}
