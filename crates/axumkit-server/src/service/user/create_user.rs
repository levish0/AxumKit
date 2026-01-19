use crate::bridge::worker_client;
use crate::repository::user::{
    repository_create_user, repository_find_user_by_email, repository_find_user_by_handle,
};
use crate::service::auth::verify_email::EmailVerificationData;
use crate::state::WorkerClient;
use crate::utils::crypto::token::generate_secure_token;
use crate::utils::redis_cache::set_json_with_ttl;
use axumkit_config::ServerConfig;
use axumkit_dto::user::{CreateUserRequest, CreateUserResponse};
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};

pub async fn service_create_user(
    conn: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    payload: CreateUserRequest,
) -> ServiceResult<CreateUserResponse> {
    let config = ServerConfig::get();
    let txn = conn.begin().await?;

    // Check if user already exists by email
    let existing_user_by_email = repository_find_user_by_email(&txn, payload.email.clone()).await?;
    if existing_user_by_email.is_some() {
        return Err(Errors::UserEmailAlreadyExists);
    }

    // Check if user already exists by handle
    let existing_user_by_handle =
        repository_find_user_by_handle(&txn, payload.handle.clone()).await?;
    if existing_user_by_handle.is_some() {
        return Err(Errors::UserHandleAlreadyExists);
    }

    // Create user in database
    let user = repository_create_user(
        &txn,
        payload.email.clone(),
        payload.handle.clone(),
        payload.display_name,
        payload.password,
    )
    .await?;

    txn.commit().await?;

    // 이메일 인증 토큰 생성 및 발송 (암호학적으로 안전한 랜덤 토큰)
    let token = generate_secure_token();
    let token_key = format!("email_verification:{}", token);

    let verification_data = EmailVerificationData {
        user_id: user.id.to_string(),
        email: user.email.clone(),
    };

    // Redis에 토큰 저장 (분 단위 → 초 단위 변환)
    let ttl_seconds = (config.auth_email_verification_token_expire_time * 60) as u64;
    set_json_with_ttl(redis_conn, &token_key, &verification_data, ttl_seconds).await?;

    // Worker 서비스에 이메일 발송 요청
    worker_client::send_verification_email(
        worker,
        &user.email,
        &user.handle,
        &token,
        config.auth_email_verification_token_expire_time as u64, // minutes
    )
    .await?;

    // MeiliSearch에 유저 인덱싱 (멘션 검색용)
    worker_client::index_user(worker, user.id).await.ok();

    Ok(CreateUserResponse {
        message: "User created successfully. Please check your email to verify your account."
            .to_string(),
    })
}
