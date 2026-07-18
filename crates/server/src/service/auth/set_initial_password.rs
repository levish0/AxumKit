use crate::repository::user::get_by_id::repository_get_user_by_id_for_update;
use crate::repository::user::{UserUpdateParams, repository_update_user};
use crate::service::auth::session::SessionService;
use crate::utils::crypto::password::hash_password;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

/// Set the first password for an OAuth-only account.
pub async fn service_set_initial_password(
    db: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    user_id: Uuid,
    session_id: &str,
    new_password: &str,
) -> ServiceResult<()> {
    let txn = db.begin().await?;

    let user = repository_get_user_by_id_for_update(&txn, user_id).await?;

    if user.password.is_some() {
        return Err(Errors::PasswordAlreadySet);
    }

    let new_password_hash = hash_password(new_password)?;

    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            password: Some(Some(new_password_hash)),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    let deleted_count =
        SessionService::delete_other_sessions(redis_conn, &user_id.to_string(), session_id).await?;

    info!(user_id = %user_id, invalidated_sessions = deleted_count, "Initial password set");

    Ok(())
}
