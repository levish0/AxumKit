use crate::permission::PermissionService;
use crate::repository::moderation::repository_create_moderation_log;
use crate::repository::user::user_bans::{
    repository_create_user_ban, repository_delete_expired_user_ban, repository_find_user_ban,
};
use crate::service::auth::session_types::SessionContext;
use chrono::{DateTime, Utc};
use constants::ModerationAction;
use dto::user::response::BanUserResponse;
use entity::common::ModerationResourceType;
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

/// Bans a user.
///
/// # Permissions
/// - Only Admin can ban users
/// - Cannot ban another Admin
///
/// # Errors
/// - Returns `Errors::UserAlreadyBanned` if the user is already banned
pub async fn service_ban_user(
    db: &DatabaseConnection,
    target_user_id: Uuid,
    expires_at: Option<DateTime<Utc>>,
    reason: String,
    session: &SessionContext,
) -> ServiceResult<BanUserResponse> {
    PermissionService::require_admin_for_target(db, Some(session), target_user_id).await?;

    let txn = db.begin().await?;

    if repository_find_user_ban(&txn, target_user_id)
        .await?
        .is_some()
    {
        return Err(Errors::UserAlreadyBanned);
    }

    repository_delete_expired_user_ban(&txn, target_user_id).await?;

    let ban = repository_create_user_ban(&txn, target_user_id, expires_at).await?;

    repository_create_moderation_log(
        &txn,
        ModerationAction::UserBan,
        Some(session.user_id),
        ModerationResourceType::User,
        Some(target_user_id),
        reason,
        Some(json!({
            "expires_at": expires_at
        })),
    )
    .await?;

    txn.commit().await?;

    info!(target_user_id = %target_user_id, actor_id = %session.user_id, "User banned");

    Ok(BanUserResponse {
        user_id: target_user_id,
        expires_at: ban.expires_at,
    })
}
