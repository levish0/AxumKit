use super::types::{DeviceLoginOutcome, DevicePendingData};
use crate::bridge::worker_client;
use crate::repository::auth_events::AUTH_EVENT_LOGIN_SUCCESS;
use crate::repository::known_devices::{
    repository_find_known_device, repository_touch_known_device,
};
use crate::service::auth::audit::{parse_ip, record_auth_event};
use crate::service::auth::session::SessionService;
use crate::state::WorkerClient;
use crate::utils::crypto::token::{generate_secure_token, hash_token};
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use config::ServerConfig;
use entity::users::Model as UserModel;
use errors::errors::Errors;
use redis::aio::ConnectionManager as RedisClient;
use sea_orm::DatabaseConnection;

/// Resolve an authenticated login against the trusted-device registry (OWASP ASVS 6.3.5).
///
/// Called after the credentials (password [+ TOTP]) have been fully verified, on **every** channel:
/// browsers present their device-recognition cookie, native apps present the same opaque token in
/// the `X-Device-Token` header. On a trusted (recognized) device it mints the session; on an
/// unrecognized device it withholds the session and emails a verification challenge, returning
/// [`DeviceLoginOutcome::VerificationRequired`]. The channel only decides how the caller transports
/// the outcome (cookie vs. response body) — the trust decision here is identical, so no channel can
/// bypass the gate.
pub async fn resolve_device_login(
    db: &DatabaseConnection,
    redis: &RedisClient,
    worker: &WorkerClient,
    user: &UserModel,
    presented_device_token: Option<String>,
    user_agent: Option<String>,
    ip_address: Option<String>,
    remember_me: bool,
) -> Result<DeviceLoginOutcome, Errors> {
    let audit_ip = parse_ip(ip_address.as_deref());

    // Trusted device? Look up the presented device token for this user.
    if let Some(token) = presented_device_token.as_deref() {
        let device_hash = hash_token(token);
        if let Some(device) = repository_find_known_device(db, user.id, &device_hash).await? {
            repository_touch_known_device(db, device, audit_ip).await?;
            let session_token =
                create_session_and_record(redis, db, user, user_agent.clone(), ip_address.clone())
                    .await?;
            return Ok(DeviceLoginOutcome::SessionCreated { session_token });
        }
    }

    // Unknown device → email challenge. Reuse the presented token if the client had one,
    // otherwise mint one to become the trusted device once confirmed.
    let device_token = presented_device_token.unwrap_or_else(generate_secure_token);
    let pending = DevicePendingData {
        user_id: user.id,
        remember_me,
        device_token,
        user_agent: user_agent.clone(),
        ip_address: ip_address.clone(),
    };

    let config = ServerConfig::get();
    let ttl_seconds = (config.auth_device_verify_token_expire_time * 60) as u64;
    // Store under the emailed token's hash so a Redis leak yields no replayable tokens.
    let verify_token = issue_token_and_store_json_with_ttl(
        redis,
        generate_secure_token,
        |token| constants::device_verify_key(&hash_token(token)),
        &pending,
        ttl_seconds,
    )
    .await?;

    let device_desc = user_agent.unwrap_or_else(|| "a new device".to_string());
    worker_client::send_device_verification(
        worker,
        &user.email,
        &user.handle,
        &device_desc,
        &verify_token,
        config.auth_device_verify_token_expire_time as u64,
    )
    .await?;

    Ok(DeviceLoginOutcome::VerificationRequired)
}

/// Create the session and record the successful-login audit event (best-effort).
async fn create_session_and_record(
    redis: &RedisClient,
    db: &DatabaseConnection,
    user: &UserModel,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<String, Errors> {
    let (raw_token, _session) = SessionService::create_session(
        redis,
        user.id.to_string(),
        user_agent.clone(),
        ip_address.clone(),
    )
    .await?;

    record_auth_event(
        db,
        Some(user.id),
        AUTH_EVENT_LOGIN_SUCCESS,
        parse_ip(ip_address.as_deref()),
        user_agent,
        None,
    )
    .await;

    Ok(raw_token)
}
