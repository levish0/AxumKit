use dto::auth::request::{
    ChangeEmailRequest, ChangePasswordRequest, CompleteSignupRequest, ConfirmEmailChangeRequest,
    ForgotPasswordRequest, LoginRequest, ResendVerificationEmailRequest, ResetPasswordRequest,
    SetInitialPasswordRequest, TotpDisableRequest, TotpEnableRequest,
    TotpRegenerateBackupCodesRequest, TotpVerifyRequest, VerifyDeviceRequest, VerifyEmailRequest,
};
use dto::auth::response::{
    AppDeviceVerifyResponse, DeviceVerificationRequiredResponse, ListSessionsResponse, SessionInfo,
    SessionTokenResponse, TotpBackupCodesResponse, TotpEnableResponse, TotpRequiredResponse,
    TotpSetupResponse, TotpStatusResponse,
};
use dto::oauth::request::google::{GoogleOneTapLoginRequest, GoogleTokenRequest};
use dto::oauth::request::{
    GithubLinkRequest, GithubLoginRequest, GithubTokenRequest, GoogleLinkRequest,
    GoogleLoginRequest, OAuthAuthorizeFlow, OAuthAuthorizeQuery, UnlinkOAuthRequest,
};
use dto::oauth::response::OAuthPendingSignupResponse;
use dto::oauth::response::{
    OAuthConnectionListResponse, OAuthConnectionResponse, OAuthUrlResponse,
};
use dto::user::{CreateUserRequest, CreateUserResponse};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::session::login::auth_login,
        super::session::login::auth_login_app,
        super::session::verify_device::auth_verify_device,
        super::session::verify_device::auth_verify_device_app,
        super::totp::verify::totp_verify_app,
        super::email::verify_email::auth_verify_email_app,
        super::session::check::auth_check,
        super::session::logout::auth_logout,
        super::session::list_sessions::auth_list_sessions,
        super::session::revoke_session::auth_revoke_session,
        super::password::forgot_password::auth_forgot_password,
        super::password::reset_password::auth_reset_password,
        super::session::signup::auth_signup,
        super::session::complete_signup::auth_complete_signup,
        super::session::complete_signup::auth_complete_signup_app,
        super::oauth::google::google_token::auth_google_token_app,
        super::oauth::github::github_token::auth_github_token_app,
        super::totp::setup::totp_setup,
        super::totp::verify::totp_verify,
        super::totp::enable::totp_enable,
        super::totp::disable::totp_disable,
        super::totp::status::totp_status,
        super::totp::regenerate_backup_codes::totp_regenerate_backup_codes,
        super::oauth::google::google_authorize::auth_google_authorize,
        super::oauth::google::google_login::auth_google_login,
        super::oauth::google::google_one_tap_login::auth_google_one_tap_login,
        super::oauth::google::google_one_tap_nonce::auth_google_one_tap_nonce,
        super::oauth::google::google_link::auth_google_link,
        super::oauth::github::github_authorize::auth_github_authorize,
        super::oauth::github::github_login::auth_github_login,
        super::oauth::github::github_link::auth_github_link,
        super::oauth::list_oauth_connections::list_oauth_connections,
        super::oauth::unlink_oauth_connection::unlink_oauth_connection,
        super::email::verify_email::auth_verify_email,
        super::email::resend_verification_email::auth_resend_verification_email,
        super::password::set_initial_password::auth_set_initial_password,
        super::password::change_password::auth_change_password,
        super::email::change_email::auth_change_email,
        super::email::confirm_email_change::auth_confirm_email_change,
    ),
    components(
        schemas(
            LoginRequest,
            VerifyEmailRequest,
            ResendVerificationEmailRequest,
            ForgotPasswordRequest,
            ResetPasswordRequest,
            CreateUserRequest,
            CreateUserResponse,
            CompleteSignupRequest,
            OAuthUrlResponse,
            OAuthPendingSignupResponse,
            OAuthAuthorizeFlow,
            OAuthAuthorizeQuery,
            GoogleLoginRequest,
            GoogleOneTapLoginRequest,
            GoogleTokenRequest,
            GithubLoginRequest,
            GithubTokenRequest,
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
            VerifyDeviceRequest,
            DeviceVerificationRequiredResponse,
            AppDeviceVerifyResponse,
            SetInitialPasswordRequest,
            ChangePasswordRequest,
            ChangeEmailRequest,
            ConfirmEmailChangeRequest,
            SessionInfo,
            ListSessionsResponse,
            SessionTokenResponse,
        )
    ),
    tags(
        (name = "Auth", description = "Authentication, signup, session, and OAuth endpoints"),
        (name = "Auth - TOTP", description = "Two-factor authentication enrollment and verification endpoints")
    )
)]
pub struct AuthApiDoc;
