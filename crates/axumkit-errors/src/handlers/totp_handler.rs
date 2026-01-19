use crate::errors::Errors;
use crate::protocol::totp::*;
use axum::http::StatusCode;
use tracing::debug;

/// TOTP 관련 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // 비즈니스 로직 에러 - debug! 레벨 (클라이언트 실수)
        Errors::TotpAlreadyEnabled
        | Errors::TotpNotEnabled
        | Errors::TotpInvalidCode
        | Errors::TotpTempTokenInvalid
        | Errors::TotpTempTokenExpired
        | Errors::TotpBackupCodeExhausted => {
            debug!("TOTP client error: {:?}", error);
        }

        // 시스템 에러 - error! 레벨
        Errors::TotpSecretGenerationFailed | Errors::TotpQrGenerationFailed => {
            tracing::error!("TOTP system error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::TotpAlreadyEnabled => Some((StatusCode::CONFLICT, TOTP_ALREADY_ENABLED, None)),
        Errors::TotpNotEnabled => Some((StatusCode::BAD_REQUEST, TOTP_NOT_ENABLED, None)),
        Errors::TotpInvalidCode => Some((StatusCode::BAD_REQUEST, TOTP_INVALID_CODE, None)),
        Errors::TotpTempTokenInvalid => {
            Some((StatusCode::BAD_REQUEST, TOTP_TEMP_TOKEN_INVALID, None))
        }
        Errors::TotpTempTokenExpired => {
            Some((StatusCode::BAD_REQUEST, TOTP_TEMP_TOKEN_EXPIRED, None))
        }
        Errors::TotpBackupCodeExhausted => {
            Some((StatusCode::UNAUTHORIZED, TOTP_BACKUP_CODE_EXHAUSTED, None))
        }
        Errors::TotpSecretGenerationFailed => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            TOTP_SECRET_GENERATION_FAILED,
            None,
        )),
        Errors::TotpQrGenerationFailed => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            TOTP_QR_GENERATION_FAILED,
            None,
        )),

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
