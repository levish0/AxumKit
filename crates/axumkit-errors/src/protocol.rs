//! Error code constants

pub mod user {
    pub const USER_INVALID_PASSWORD: &str = "user:invalid_password";
    pub const USER_PASSWORD_NOT_SET: &str = "user:password_not_set";
    pub const USER_INVALID_SESSION: &str = "user:invalid_session";
    pub const USER_NOT_VERIFIED: &str = "user:not_verified";
    pub const USER_NOT_FOUND: &str = "user:not_found";
    pub const USER_UNAUTHORIZED: &str = "user:unauthorized";
    pub const USER_BANNED: &str = "user:banned";
    pub const USER_PERMISSION_INSUFFICIENT: &str = "user:permission_insufficient";
    pub const USER_HANDLE_ALREADY_EXISTS: &str = "user:handle_already_exists";
    pub const USER_EMAIL_ALREADY_EXISTS: &str = "user:email_already_exists";
    pub const USER_TOKEN_EXPIRED: &str = "user:token_expired";
    pub const USER_NO_REFRESH_TOKEN: &str = "user:no_refresh_token";
    pub const USER_INVALID_TOKEN: &str = "user:invalid_token";
}

pub mod post {
    pub const POST_NOT_FOUND: &str = "post:not_found";
}

pub mod oauth {
    pub const OAUTH_INVALID_AUTH_URL: &str = "oauth:invalid_auth_url";
    pub const OAUTH_INVALID_TOKEN_URL: &str = "oauth:invalid_token_url";
    pub const OAUTH_INVALID_REDIRECT_URL: &str = "oauth:invalid_redirect_url";
    pub const OAUTH_TOKEN_EXCHANGE_FAILED: &str = "oauth:token_exchange_failed";
    pub const OAUTH_USER_INFO_FETCH_FAILED: &str = "oauth:user_info_fetch_failed";
    pub const OAUTH_USER_INFO_PARSE_FAILED: &str = "oauth:user_info_parse_failed";
    pub const OAUTH_ACCOUNT_ALREADY_LINKED: &str = "oauth:account_already_linked";
    pub const OAUTH_CONNECTION_NOT_FOUND: &str = "oauth:connection_not_found";
    pub const OAUTH_CANNOT_UNLINK_LAST_CONNECTION: &str = "oauth:cannot_unlink_last_connection";
    pub const OAUTH_INVALID_IMAGE_URL: &str = "oauth:invalid_image_url";
    pub const OAUTH_INVALID_STATE: &str = "oauth:invalid_state";
    pub const OAUTH_STATE_EXPIRED: &str = "oauth:state_expired";
    pub const OAUTH_HANDLE_REQUIRED: &str = "oauth:handle_required";
    pub const OAUTH_EMAIL_ALREADY_EXISTS: &str = "oauth:email_already_exists";
    pub const OAUTH_EMAIL_NOT_VERIFIED: &str = "oauth:email_not_verified";
}

pub mod general {
    pub const BAD_REQUEST: &str = "general:bad_request";
    pub const VALIDATION_ERROR: &str = "general:validation_error";
    pub const INVALID_IP_ADDRESS: &str = "general:invalid_ip_address";
    pub const FORBIDDEN: &str = "FORBIDDEN";
    pub const FILE_TOO_LARGE: &str = "FILE_TOO_LARGE";
}

pub mod file {
    pub const FILE_UPLOAD_ERROR: &str = "file:upload_error";
    pub const FILE_NOT_FOUND: &str = "file:not_found";
    pub const FILE_READ_ERROR: &str = "file:read_error";
}

pub mod password {
    pub const PASSWORD_REQUIRED_FOR_UPDATE: &str = "password:required_for_update";
    pub const PASSWORD_INCORRECT: &str = "password:incorrect";
    pub const PASSWORD_CANNOT_UPDATE_OAUTH_ONLY: &str = "password:cannot_update_oauth_only";
    pub const PASSWORD_NEW_PASSWORD_MISSING: &str = "password:new_password_missing";
    pub const PASSWORD_ALREADY_SET: &str = "password:already_set";
}

pub mod token {
    pub const TOKEN_INVALID_VERIFICATION: &str = "token:invalid_verification";
    pub const TOKEN_EXPIRED_VERIFICATION: &str = "token:expired_verification";
    pub const TOKEN_EMAIL_MISMATCH: &str = "token:email_mismatch";
    pub const TOKEN_INVALID_RESET: &str = "token:invalid_reset";
    pub const TOKEN_EXPIRED_RESET: &str = "token:expired_reset";
    pub const TOKEN_INVALID_EMAIL_CHANGE: &str = "token:invalid_email_change";
}

pub mod email {
    pub const EMAIL_ALREADY_VERIFIED: &str = "email:already_verified";
}

pub mod session {
    pub const SESSION_INVALID_USER_ID: &str = "session:invalid_user_id";
    pub const SESSION_EXPIRED: &str = "session:expired";
    pub const SESSION_NOT_FOUND: &str = "session:not_found";
}

pub mod system {
    pub const SYS_INTERNAL_ERROR: &str = "system:internal_error";
    pub const SYS_HASHING_ERROR: &str = "system:hashing_error";
    pub const SYS_NOT_FOUND: &str = "system:not_found";
    pub const SYS_TRANSACTION_ERROR: &str = "system:transaction_error";
    pub const SYS_DATABASE_ERROR: &str = "system:database_error";
    pub const SYS_TOKEN_CREATION_ERROR: &str = "system:token_creation_error";
}

pub mod worker {
    pub const WORKER_CONNECTION_FAILED: &str = "worker:connection_failed";
    pub const WORKER_RESPONSE_INVALID: &str = "worker:response_invalid";
    pub const VERIFICATION_EMAIL_SEND_FAILED: &str = "worker:verification_email_send_failed";
    pub const PASSWORD_RESET_EMAIL_SEND_FAILED: &str = "worker:password_reset_email_send_failed";
}

pub mod eventstream {
    pub const EVENTSTREAM_PUBLISH_FAILED: &str = "eventstream:publish_failed";
}

pub mod rate_limit {
    pub const RATE_LIMIT_EXCEEDED: &str = "rate_limit:exceeded";
}

pub mod turnstile {
    pub const TURNSTILE_TOKEN_MISSING: &str = "turnstile:token_missing";
    pub const TURNSTILE_VERIFICATION_FAILED: &str = "turnstile:verification_failed";
    pub const TURNSTILE_SERVICE_ERROR: &str = "turnstile:service_error";
}

pub mod meilisearch {
    pub const MEILISEARCH_QUERY_FAILED: &str = "meilisearch:query_failed";
}

pub mod totp {
    pub const TOTP_ALREADY_ENABLED: &str = "totp:already_enabled";
    pub const TOTP_NOT_ENABLED: &str = "totp:not_enabled";
    pub const TOTP_INVALID_CODE: &str = "totp:invalid_code";
    pub const TOTP_TEMP_TOKEN_INVALID: &str = "totp:temp_token_invalid";
    pub const TOTP_TEMP_TOKEN_EXPIRED: &str = "totp:temp_token_expired";
    pub const TOTP_BACKUP_CODE_EXHAUSTED: &str = "totp:backup_code_exhausted";
    pub const TOTP_SECRET_GENERATION_FAILED: &str = "totp:secret_generation_failed";
    pub const TOTP_QR_GENERATION_FAILED: &str = "totp:qr_generation_failed";
}
