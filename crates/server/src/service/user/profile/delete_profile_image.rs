use crate::connection::r2_assets_conn::R2AssetsClient;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::service::auth::session_types::SessionContext;
use crate::service::blob_cleanup::delete_user_image_blob_if_unreferenced;
use crate::service::user::utils::spawn_index_user;
use crate::state::WorkerClient;
use errors::errors::Errors;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;

/// Deletes the current user's profile image.
///
/// # Role
/// - Clears `profile_image` in the DB within a transaction.
/// - After commit, deletes the R2 object on a best-effort basis.
/// - Triggers a search index update.
///
/// # Related
/// - `repository_get_user_by_id`
/// - `repository_update_user`
/// - `worker_client::index_user`
///
/// # Errors
/// - `Errors::NotFound` if there is no profile image
/// - Returns a repository error if the DB update fails.
pub async fn service_delete_profile_image(
    db: &DatabaseConnection,
    r2_assets: &R2AssetsClient,
    worker: &WorkerClient,
    session: &SessionContext,
) -> Result<(), Errors> {
    let txn = db.begin().await?;

    let user = repository_get_user_by_id(&txn, session.user_id).await?;

    let Some(storage_key) = user.profile_image else {
        return Err(Errors::NotFound("No profile image to delete".to_string()));
    };

    // Update the DB first (within the transaction)
    repository_update_user(
        &txn,
        session.user_id,
        UserUpdateParams {
            profile_image: Some(None),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    // R2 delete happens outside the transaction (best effort). Content-addressed image may be shared with
    // other users, so only delete the object when nothing references it anymore.
    delete_user_image_blob_if_unreferenced(db, r2_assets, &storage_key).await;

    // Index the user in MeiliSearch (reflect the profile image deletion)
    spawn_index_user(worker, session.user_id);

    info!(user_id = %session.user_id, "Profile image deleted");

    Ok(())
}
