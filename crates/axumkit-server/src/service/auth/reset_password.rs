use crate::repository::user::{UserUpdateParams, repository_update_user};
use crate::service::auth::forgot_password::PasswordResetData;
use crate::service::auth::session::SessionService;
use crate::utils::crypto::password::hash_password;
use crate::utils::redis_cache::get_json_and_delete;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use tracing::info;
use uuid::Uuid;

///
/// # Arguments
pub async fn service_reset_password<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    token: &str,
    new_password: &str,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    let token_key = axumkit_constants::password_reset_key(token);
    let reset_data: PasswordResetData = get_json_and_delete(
        redis_conn,
        &token_key,
        || Errors::TokenInvalidReset,
        |_| Errors::TokenInvalidReset,
    )
    .await?;

    let user_id = Uuid::parse_str(&reset_data.user_id).map_err(|_| Errors::TokenInvalidReset)?;

    let password_hash = hash_password(new_password)?;

    repository_update_user(
        conn,
        user_id,
        UserUpdateParams {
            password: Some(Some(password_hash)),
            ..Default::default()
        },
    )
    .await?;

    let deleted_count =
        SessionService::delete_all_user_sessions(redis_conn, &user_id.to_string()).await?;

    info!(user_id = %user_id, invalidated_sessions = deleted_count, "Password reset completed");

    Ok(())
}
