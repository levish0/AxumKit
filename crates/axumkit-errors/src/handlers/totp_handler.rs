use crate::errors::Errors;
use crate::protocol::totp::*;
use axum::http::StatusCode;
use tracing::debug;

/// TOTP error logging handler
pub fn log_error(error: &Errors) {
    match error {
        // Business logic errors - debug! level (client mistakes)
        Errors::TotpAlreadyEnabled
        | Errors::TotpNotEnabled
        | Errors::TotpInvalidCode
        | Errors::TotpTempTokenInvalid
        | Errors::TotpTempTokenExpired
        | Errors::TotpBackupCodeExhausted => {
            debug!("TOTP client error: {:?}", error);
        }

        // System errors - error! level
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

        _ => None, // Return None for errors from other domains
    }
}
