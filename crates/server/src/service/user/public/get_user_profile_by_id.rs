use crate::repository::user::get_by_id::repository_get_user_by_id;
use crate::repository::user::user_roles::repository_find_user_roles;
use crate::service::user::bans::find_active_user_ban;
use crate::service::user::mapper::user_to_public_profile;
use dto::user::PublicUserProfile;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

/// Fetches a public user profile by user ID.
///
/// # Responsibilities
/// Combines the user's basic info with their highest role name and returns the public profile DTO.
///
/// # Related
/// - `repository_get_user_by_id`
/// - `repository_find_user_roles`
///
/// # Errors
/// - `Errors::UserNotFound` if the user does not exist
/// - DB/repository errors on lookup failure.
pub async fn service_get_user_profile_by_id(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> ServiceResult<PublicUserProfile> {
    let user = repository_get_user_by_id(db, user_id).await?;
    let roles = repository_find_user_roles(db, user.id).await?;
    let ban = find_active_user_ban(db, user.id).await?;

    Ok(user_to_public_profile(user, roles, ban))
}
