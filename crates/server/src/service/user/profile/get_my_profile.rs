use crate::repository::user::repository_get_user_by_id;
use crate::repository::user::user_roles::repository_find_user_roles;
use crate::service::auth::session_types::SessionContext;
use crate::service::user::bans::find_active_user_ban;
use crate::utils::r2_url::build_r2_public_url;
use dto::user::UserResponse;
use errors::errors::Errors;
use sea_orm::DatabaseConnection;

/// Fetches the currently logged-in user's profile.
///
/// # Responsibilities
/// Combines the user's basic info with their highest role name to build the my-profile response.
///
/// # Related
/// - `repository_get_user_by_id`
/// - `repository_find_user_roles`
///
/// # Errors
/// - `Errors::UserNotFound` if the user does not exist
/// - DB/repository errors on lookup failure.
pub async fn service_get_my_profile(
    db: &DatabaseConnection,
    session: &SessionContext,
) -> Result<UserResponse, Errors> {
    let user = repository_get_user_by_id(db, session.user_id).await?;
    let roles = repository_find_user_roles(db, session.user_id).await?;
    let is_banned = find_active_user_ban(db, session.user_id).await?.is_some();

    let response = UserResponse {
        id: session.user_id,
        email: user.email,
        handle: user.handle,
        display_name: user.display_name,
        bio: user.bio,
        profile_image: user.profile_image.as_deref().map(build_r2_public_url),
        banner_image: user.banner_image.as_deref().map(build_r2_public_url),
        roles,
        is_banned,
        has_password: user.password.is_some(),
        created_at: user.created_at,
    };

    Ok(response)
}
