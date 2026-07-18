use crate::repository::user::get_by_handle::repository_get_user_by_handle;
use crate::repository::user::user_roles::repository_find_user_roles;
use crate::service::user::bans::find_active_user_ban;
use crate::service::user::mapper::user_to_public_profile;
use dto::user::PublicUserProfile;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

/// Fetches a public user profile by handle.
///
/// # Responsibilities
/// Combines the user's basic info with their highest role name and returns the public profile DTO.
///
/// # Related
/// - `repository_get_user_by_handle`
/// - `repository_find_user_roles`
///
/// # Errors
/// - `Errors::UserNotFound` if the user does not exist
/// - DB/repository errors on lookup failure.
pub async fn service_get_user_profile_by_handle(
    db: &DatabaseConnection,
    handle: &str,
) -> ServiceResult<PublicUserProfile> {
    let user = repository_get_user_by_handle(db, handle.to_string()).await?;
    let roles = repository_find_user_roles(db, user.id).await?;
    let ban = find_active_user_ban(db, user.id).await?;

    Ok(user_to_public_profile(user, roles, ban))
}
