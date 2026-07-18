use sea_orm::DatabaseConnection;

use super::cleanup_expired_roles::run_cleanup_expired_roles;
use super::cleanup_old_notifications::run_cleanup_old_notifications;
use super::expiry::run_batched_expiry_delete;
use entity::{group_members, user_bans};

/// Batch size for cleanup operations
const BATCH_SIZE: u64 = 1000;

/// Notification retention days (notifications older than this are deleted)
const NOTIFICATION_RETENTION_DAYS: u32 = 90;

/// Run the cleanup job
///
/// Cleans up:
/// - Expired ACL group members (expires_at < NOW())
/// - Expired user roles (expires_at < NOW())
/// - Expired user bans (expires_at < NOW())
/// - Old notifications (created_at < NOW() - retention_days)
pub async fn run_cleanup(db: &DatabaseConnection) {
    tracing::info!("Starting scheduled cleanup job");

    // 1. Cleanup expired ACL group members
    let group_members = run_batched_expiry_delete(
        db,
        group_members::Entity,
        group_members::Column::Id,
        group_members::Column::ExpiresAt,
        "agm",
        "group_members",
        BATCH_SIZE,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to cleanup expired ACL group members");
        0
    });

    // 2. Cleanup expired roles
    let roles = match run_cleanup_expired_roles(db, BATCH_SIZE).await {
        Ok((count, _user_ids)) => count,
        Err(e) => {
            tracing::error!(error = %e, "Failed to cleanup expired user roles");
            0
        }
    };

    // 3. Cleanup expired user bans (read paths already filter expiry; this
    // reclaims the dead rows)
    let bans = run_batched_expiry_delete(
        db,
        user_bans::Entity,
        user_bans::Column::Id,
        user_bans::Column::ExpiresAt,
        "ub",
        "user_bans",
        BATCH_SIZE,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to cleanup expired user bans");
        0
    });

    // 4. Cleanup old notifications
    let notifs = run_cleanup_old_notifications(db, NOTIFICATION_RETENTION_DAYS, BATCH_SIZE)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Failed to cleanup old notifications");
            0
        });

    tracing::info!(
        expired_group_members = group_members,
        expired_roles = roles,
        expired_bans = bans,
        old_notifications = notifs,
        "Cleanup job completed"
    );
}
