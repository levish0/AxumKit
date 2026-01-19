use axumkit_dto::auth::request::{
    ChangeEmailRequest, ChangePasswordRequest, CompleteSignupRequest, ConfirmEmailChangeRequest,
    ForgotPasswordRequest, LoginRequest, ResetPasswordRequest, TotpDisableRequest,
    TotpEnableRequest, TotpRegenerateBackupCodesRequest, TotpVerifyRequest, VerifyEmailRequest,
};
use axumkit_dto::auth::response::{
    TotpBackupCodesResponse, TotpEnableResponse, TotpRequiredResponse, TotpSetupResponse,
    TotpStatusResponse,
};
use axumkit_dto::oauth::request::{
    GithubLinkRequest, GithubLoginRequest, GoogleLinkRequest, GoogleLoginRequest,
    UnlinkOAuthRequest,
};
use axumkit_dto::oauth::response::OAuthPendingSignupResponse;
use axumkit_dto::oauth::response::{
    OAuthConnectionListResponse, OAuthConnectionResponse, OAuthUrlResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::login::auth_login,
        super::logout::auth_logout,
        super::forgot_password::auth_forgot_password,
        super::reset_password::auth_reset_password,
        super::complete_signup::auth_complete_signup,
        super::totp::setup::totp_setup,
        super::totp::verify::totp_verify,
        super::totp::enable::totp_enable,
        super::totp::disable::totp_disable,
        super::totp::status::totp_status,
        super::totp::regenerate_backup_codes::totp_regenerate_backup_codes,
        super::oauth::google::google_authorize::auth_google_authorize,
        super::oauth::google::google_login::auth_google_login,
        super::oauth::google::google_link::auth_google_link,
        super::oauth::github::github_authorize::auth_github_authorize,
        super::oauth::github::github_login::auth_github_login,
        super::oauth::github::github_link::auth_github_link,
        super::oauth::list_oauth_connections::list_oauth_connections,
        super::oauth::unlink_oauth_connection::unlink_oauth_connection,
        super::verify_email::auth_verify_email,
        super::resend_verification_email::auth_resend_verification_email,
        super::change_password::auth_change_password,
        super::change_email::auth_change_email,
        super::confirm_email_change::auth_confirm_email_change,
    ),
    components(
        schemas(
            LoginRequest,
            VerifyEmailRequest,
            ForgotPasswordRequest,
            ResetPasswordRequest,
            CompleteSignupRequest,
            OAuthUrlResponse,
            OAuthPendingSignupResponse,
            GoogleLoginRequest,
            GithubLoginRequest,
            GoogleLinkRequest,
            GithubLinkRequest,
            UnlinkOAuthRequest,
            OAuthConnectionResponse,
            OAuthConnectionListResponse,
            TotpVerifyRequest,
            TotpEnableRequest,
            TotpDisableRequest,
            TotpRegenerateBackupCodesRequest,
            TotpSetupResponse,
            TotpStatusResponse,
            TotpEnableResponse,
            TotpBackupCodesResponse,
            TotpRequiredResponse,
            ChangePasswordRequest,
            ChangeEmailRequest,
            ConfirmEmailChangeRequest,
        )
    ),
    tags(
        (name = "Auth", description = "Authentication endpoints")
    )
)]
pub struct AuthApiDoc;
