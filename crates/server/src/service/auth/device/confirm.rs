use super::types::DevicePendingData;
use crate::repository::auth_events::{AUTH_EVENT_LOGIN_SUCCESS, AUTH_EVENT_NEW_DEVICE};
use crate::repository::known_devices::{
    repository_find_known_device, repository_register_known_device,
};
use crate::repository::user::repository_find_user_by_id;
use crate::service::auth::audit::{parse_ip, record_auth_event};
use crate::service::auth::session::SessionService;
use crate::utils::crypto::token::hash_token;
use crate::utils::redis_cache::get_json_and_delete;
use errors::errors::Errors;
use redis::aio::ConnectionManager as RedisClient;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

/// Result of confirming a new-device verification: the caller sets both the session cookie and the
/// (now trusted) device cookie.
pub struct DeviceVerifyResult {
    pub session_token: String,
    pub device_token: String,
    pub remember_me: bool,
}

/// Confirm a pending new-device verification via the emailed single-use token.
///
/// Registers the device as trusted, mints the session, and records the audit events. The emailed
/// token is the proof, so no session is required to call this.
pub async fn confirm_device_verification(
    db: &DatabaseConnection,
    redis: &RedisClient,
    token: &str,
) -> Result<DeviceVerifyResult, Errors> {
    // Single-use lookup by the hashed token id.
    let key = constants::device_verify_key(&hash_token(token));
    let pending: DevicePendingData = get_json_and_delete(
        redis,
        &key,
        || Errors::TokenInvalidDeviceVerify,
        |_| Errors::TokenInvalidDeviceVerify,
    )
    .await?;

    let user_id: Uuid = pending.user_id;
    let user = repository_find_user_by_id(db, user_id)
        .await?
        .ok_or(Errors::UserNotFound)?;

    let device_hash = hash_token(&pending.device_token);
    let device_ip = parse_ip(pending.ip_address.as_deref());

    // Register the device as trusted (idempotent: a concurrent confirm may have already added it).
    if repository_find_known_device(db, user_id, &device_hash)
        .await?
        .is_none()
    {
        repository_register_known_device(
            db,
            user_id,
            device_hash,
            pending.user_agent.clone(),
            device_ip,
        )
        .await?;
    }

    // Mint the session that was withheld at login.
    let (session_token, _session) = SessionService::create_session(
        redis,
        user_id.to_string(),
        pending.user_agent.clone(),
        pending.ip_address.clone(),
    )
    .await?;

    record_auth_event(
        db,
        Some(user_id),
        AUTH_EVENT_NEW_DEVICE,
        device_ip,
        pending.user_agent.clone(),
        None,
    )
    .await;
    record_auth_event(
        db,
        Some(user_id),
        AUTH_EVENT_LOGIN_SUCCESS,
        device_ip,
        pending.user_agent,
        None,
    )
    .await;

    // `user` is only needed to confirm the account still exists; drop it explicitly.
    let _ = user;

    Ok(DeviceVerifyResult {
        session_token,
        device_token: pending.device_token,
        remember_me: pending.remember_me,
    })
}
