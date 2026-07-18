use crate::repository::user::user_roles::repository_find_user_roles;
use crate::repository::user::{
    UserUpdateParams, repository_find_user_by_handle, repository_update_user,
};
use crate::service::auth::session_types::SessionContext;
use crate::service::user::bans::find_active_user_ban;
use crate::service::user::utils::spawn_index_user;
use crate::state::WorkerClient;
use crate::utils::r2_url::build_r2_public_url;
use dto::user::UserResponse;
use dto::user::request::UpdateMyProfileRequest;
use errors::errors::Errors;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;

/// Updates the currently logged-in user's profile information.
///
/// # Responsibilities
/// - Updates the handle, display name, and bio.
/// - Checks for duplicates when the handle changes.
/// - Returns the latest profile response including role names.
/// - Triggers a search index refresh after commit.
///
/// # Related
/// - `repository_update_user`
/// - `repository_find_user_by_handle`
/// - `repository_find_user_roles`
/// - `worker_client::index_user`
///
/// # Errors
/// - Returns `UserHandleAlreadyExists` if the handle is already taken.
/// - DB/repository errors on update failure.
pub async fn service_update_my_profile(
    db: &DatabaseConnection,
    worker: &WorkerClient,
    session: &SessionContext,
    request: UpdateMyProfileRequest,
) -> Result<UserResponse, Errors> {
    let txn = db.begin().await?;

    if let Some(ref handle) = request.handle
        && let Some(existing) = repository_find_user_by_handle(&txn, handle.clone()).await?
        && existing.id != session.user_id
    {
        return Err(Errors::UserHandleAlreadyExists);
    }

    let params = UserUpdateParams {
        handle: request.handle,
        display_name: request.display_name,
        bio: request.bio,
        ..Default::default()
    };

    let updated_user = repository_update_user(&txn, session.user_id, params).await?;
    let roles = repository_find_user_roles(&txn, session.user_id).await?;
    let is_banned = find_active_user_ban(&txn, session.user_id).await?.is_some();

    txn.commit().await?;

    info!(user_id = %session.user_id, "Profile updated");

    // Index the user in MeiliSearch (reflect profile changes)
    spawn_index_user(worker, session.user_id);

    Ok(UserResponse {
        id: session.user_id,
        email: updated_user.email,
        handle: updated_user.handle,
        display_name: updated_user.display_name,
        bio: updated_user.bio,
        profile_image: updated_user
            .profile_image
            .as_deref()
            .map(build_r2_public_url),
        banner_image: updated_user
            .banner_image
            .as_deref()
            .map(build_r2_public_url),
        roles,
        is_banned,
        has_password: updated_user.password.is_some(),
        created_at: updated_user.created_at,
    })
}
