use crate::repository::user::repository_get_user_by_id;
use axumkit_dto::auth::response::TotpStatusResponse;
use axumkit_errors::errors::ServiceResult;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

pub async fn service_totp_status<C>(conn: &C, user_id: Uuid) -> ServiceResult<TotpStatusResponse>
where
    C: ConnectionTrait,
{
    let user = repository_get_user_by_id(conn, user_id).await?;

    let enabled = user.totp_enabled_at.is_some();

    Ok(TotpStatusResponse {
        enabled,
        enabled_at: user.totp_enabled_at,
        backup_codes_remaining: if enabled {
            Some(user.totp_backup_codes.map(|c| c.len()).unwrap_or(0))
        } else {
            None
        },
    })
}
