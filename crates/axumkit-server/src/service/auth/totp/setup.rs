use super::common::ISSUER;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use axumkit_dto::auth::response::TotpSetupResponse;
use axumkit_errors::errors::{Errors, ServiceResult};
use rand::Rng;
use sea_orm::ConnectionTrait;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

/// TOTP 설정 시작: secret 생성, DB 저장 (아직 활성화 안 함), QR 반환
pub async fn service_totp_setup<C>(
    conn: &C,
    user_id: Uuid,
    email: &str,
) -> ServiceResult<TotpSetupResponse>
where
    C: ConnectionTrait,
{
    // 사용자 조회
    let user = repository_get_user_by_id(conn, user_id).await?;

    // 이미 TOTP 활성화된 경우
    if user.totp_enabled_at.is_some() {
        return Err(Errors::TotpAlreadyEnabled);
    }

    // Secret 생성 (20 bytes = 160 bits, RFC 4226 권장)
    let (secret_bytes, secret_base32) = {
        let mut rng = rand::rng();
        let bytes: [u8; 20] = rng.random();
        let secret = Secret::Raw(bytes.to_vec());
        (bytes, secret.to_encoded().to_string())
    };

    // TOTP 객체 생성
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,  // digits
        1,  // skew
        30, // step
        secret_bytes.to_vec(),
        Some(ISSUER.to_string()),
        email.to_string(),
    )
    .map_err(|_| Errors::TotpSecretGenerationFailed)?;

    // QR 코드 생성 (PNG base64)
    let qr_code_uri = totp.get_url();
    let qr_code_png_base64 = totp
        .get_qr_base64()
        .map_err(|_| Errors::TotpQrGenerationFailed)?;

    // DB에 secret 저장 (totp_enabled_at은 아직 NULL)
    repository_update_user(
        conn,
        user_id,
        UserUpdateParams {
            totp_secret: Some(Some(secret_base32)),
            ..Default::default()
        },
    )
    .await?;

    Ok(TotpSetupResponse {
        qr_code_base64: qr_code_png_base64,
        qr_code_uri,
    })
}
