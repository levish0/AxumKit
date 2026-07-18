use crate::connection::r2_assets_conn::R2AssetsClient;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::service::auth::session_types::SessionContext;
use crate::service::blob_cleanup::delete_user_image_blob_if_unreferenced;
use errors::errors::Errors;
use sea_orm::DatabaseConnection;
use tracing::info;

/// Deletes the current user's banner image.
///
/// # Role
/// - Looks up the user's banner image key.
/// - Attempts to delete the R2 object and sets `banner_image` to `NULL` in the DB.
///
/// # Related
/// - `repository_get_user_by_id`
/// - `repository_update_user`
/// - `R2AssetsClient::delete`
///
/// # Errors
/// - `Errors::NotFound` if there is no banner image
/// - Returns a repository error if the DB update fails.
pub async fn service_delete_banner_image(
    db: &DatabaseConnection,
    r2_assets: &R2AssetsClient,
    session: &SessionContext,
) -> Result<(), Errors> {
    let user = repository_get_user_by_id(db, session.user_id).await?;

    let Some(storage_key) = user.banner_image else {
        return Err(Errors::NotFound("No banner image to delete".to_string()));
    };

    // Clear the DB reference first so the delete below sees the up-to-date state.
    repository_update_user(
        db,
        session.user_id,
        UserUpdateParams {
            banner_image: Some(None),
            ..Default::default()
        },
    )
    .await?;

    // R2 delete is best effort. Content-addressed image may be shared with other users,
    // so only delete the object when nothing references it anymore.
    delete_user_image_blob_if_unreferenced(db, r2_assets, &storage_key).await;

    info!(user_id = %session.user_id, "Banner image deleted");

    Ok(())
}
