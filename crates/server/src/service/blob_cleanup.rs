//! Safe deletion of content-addressed R2 assets.
//!
//! User images are keyed by content hash, so identical uploads share a single R2
//! object. Deleting such an object unconditionally can destroy a blob that another
//! user still references. Every asset delete goes
//! through the helpers here, which remove the object only when it is confirmed
//! unreferenced.

use crate::connection::R2AssetsClient;
use crate::repository::user::repository_is_user_image_referenced;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;
use tracing::{debug, warn};

/// Shared gate: physically delete the R2 object only when confirmed unreferenced.
///
/// Best-effort and fail-safe — on a still-referenced blob, or if the reference check
/// itself fails, the object is kept (the periodic orphan cleanup reclaims genuine
/// leaks later). Never deletes a live, shared asset.
async fn delete_blob_if_unreferenced(
    r2_assets: &R2AssetsClient,
    storage_key: &str,
    referenced: Result<bool, Errors>,
) {
    match referenced {
        Ok(true) => debug!(storage_key, "Blob still referenced; keeping R2 object"),
        Ok(false) => {
            if let Err(e) = r2_assets.delete(storage_key).await {
                warn!(storage_key, error = ?e, "Failed to delete unreferenced blob from R2");
            }
        }
        Err(e) => {
            warn!(storage_key, error = ?e, "Skipping blob delete; reference check failed")
        }
    }
}

/// Delete a user image asset only if no user still references it as a profile/banner image.
pub async fn delete_user_image_blob_if_unreferenced<C>(
    conn: &C,
    r2_assets: &R2AssetsClient,
    storage_key: &str,
) where
    C: ConnectionTrait,
{
    let referenced = repository_is_user_image_referenced(conn, storage_key).await;
    delete_blob_if_unreferenced(r2_assets, storage_key, referenced).await;
}
