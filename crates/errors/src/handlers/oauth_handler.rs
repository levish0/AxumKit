use crate::errors::Errors;
use crate::protocol::oauth::*;
use axum::http::StatusCode;
use tracing::{debug, error, warn};

/// OAuth domain error logging.
pub fn log_error(error: &Errors) {
    match error {
        // System-severity errors - error! level
        Errors::OauthUserInfoParseFailed(msg) => {
            error!(details = %msg, "OAuth user info parse failed");
        }

        // OAuth errors - warn! level (external-service related)
        Errors::OauthInvalidAuthUrl
        | Errors::OauthInvalidTokenUrl
        | Errors::OauthInvalidRedirectUrl
        | Errors::OauthTokenExchangeFailed
        | Errors::OauthUserInfoFetchFailed
        | Errors::GoogleJwksFetchFailed
        | Errors::GoogleJwksParseFailed => {
            warn!(error = ?error, "OAuth error");
        }

        // Client-side/business validation errors - debug! level
        Errors::OauthAccountAlreadyLinked
        | Errors::OauthConnectionNotFound
        | Errors::OauthCannotUnlinkLastConnection
        | Errors::OauthInvalidImageUrl
        | Errors::OauthInvalidState
        | Errors::OauthStateExpired
        | Errors::OauthHandleRequired
        | Errors::OauthEmailAlreadyExists
        | Errors::OauthEmailNotVerified
        | Errors::GoogleInvalidIdToken
        | Errors::GoogleOneTapNonceInvalid
        | Errors::GithubInvalidToken => {
            debug!(error = ?error, "Client error");
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::OauthInvalidAuthUrl => {
            Some((StatusCode::BAD_REQUEST, OAUTH_INVALID_AUTH_URL, None))
        }
        Errors::OauthInvalidTokenUrl => {
            Some((StatusCode::BAD_REQUEST, OAUTH_INVALID_TOKEN_URL, None))
        }
        Errors::OauthInvalidRedirectUrl => {
            Some((StatusCode::BAD_REQUEST, OAUTH_INVALID_REDIRECT_URL, None))
        }
        Errors::OauthTokenExchangeFailed => {
            Some((StatusCode::BAD_REQUEST, OAUTH_TOKEN_EXCHANGE_FAILED, None))
        }
        Errors::OauthUserInfoFetchFailed => {
            Some((StatusCode::BAD_REQUEST, OAUTH_USER_INFO_FETCH_FAILED, None))
        }
        Errors::OauthUserInfoParseFailed(msg) => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            OAUTH_USER_INFO_PARSE_FAILED,
            Some(msg.clone()),
        )),
        Errors::OauthAccountAlreadyLinked => {
            Some((StatusCode::CONFLICT, OAUTH_ACCOUNT_ALREADY_LINKED, None))
        }
        Errors::OauthConnectionNotFound => {
            Some((StatusCode::NOT_FOUND, OAUTH_CONNECTION_NOT_FOUND, None))
        }
        Errors::OauthCannotUnlinkLastConnection => Some((
            StatusCode::BAD_REQUEST,
            OAUTH_CANNOT_UNLINK_LAST_CONNECTION,
            None,
        )),
        Errors::OauthInvalidImageUrl => {
            Some((StatusCode::BAD_REQUEST, OAUTH_INVALID_IMAGE_URL, None))
        }
        Errors::OauthInvalidState => Some((StatusCode::BAD_REQUEST, OAUTH_INVALID_STATE, None)),
        Errors::OauthStateExpired => Some((StatusCode::BAD_REQUEST, OAUTH_STATE_EXPIRED, None)),
        Errors::OauthHandleRequired => Some((StatusCode::BAD_REQUEST, OAUTH_HANDLE_REQUIRED, None)),
        Errors::OauthEmailAlreadyExists => {
            Some((StatusCode::CONFLICT, OAUTH_EMAIL_ALREADY_EXISTS, None))
        }
        Errors::OauthEmailNotVerified => {
            Some((StatusCode::BAD_REQUEST, OAUTH_EMAIL_NOT_VERIFIED, None))
        }
        Errors::GoogleInvalidIdToken => {
            Some((StatusCode::BAD_REQUEST, GOOGLE_INVALID_ID_TOKEN, None))
        }
        Errors::GoogleOneTapNonceInvalid => {
            Some((StatusCode::BAD_REQUEST, GOOGLE_ONE_TAP_NONCE_INVALID, None))
        }
        Errors::GithubInvalidToken => Some((StatusCode::BAD_REQUEST, GITHUB_INVALID_TOKEN, None)),
        Errors::GoogleJwksFetchFailed => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            GOOGLE_JWKS_FETCH_FAILED,
            None,
        )),
        Errors::GoogleJwksParseFailed => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            GOOGLE_JWKS_PARSE_FAILED,
            None,
        )),

        _ => None, // Return None for errors from other domains
    }
}
