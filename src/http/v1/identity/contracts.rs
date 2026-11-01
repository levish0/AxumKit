use crate::http::validation;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError, ValidationErrors};

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct EmailInput {
    #[validate(custom(function = "validation::email"))]
    pub email: String,
    /// Cloudflare Turnstile token. The widget action is `email_verification` for
    /// verification mail or `password_reset` for password reset. Required when
    /// the server enables human verification; ignored otherwise.
    #[validate(length(max = 2048))]
    #[schema(max_length = 2048)]
    pub human_verification_token: Option<String>,
}

#[derive(Default, Deserialize, ToSchema, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum SessionTransport {
    #[default]
    Cookie,
    Bearer,
}

#[derive(Deserialize, ToSchema)]
#[serde(
    tag = "kind",
    content = "code",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MfaProofInput {
    Totp(String),
    RecoveryCode(String),
}
impl Validate for MfaProofInput {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let code = match self {
            Self::Totp(code) | Self::RecoveryCode(code) => code,
        };
        if code.is_empty() || code.len() > 1024 {
            let mut errors = ValidationErrors::new();
            errors.add("code", ValidationError::new("length"));
            return Err(errors);
        }
        Ok(())
    }
}
impl From<MfaProofInput> for application::identity::MfaProof {
    fn from(proof: MfaProofInput) -> Self {
        match proof {
            MfaProofInput::Totp(code) => Self::Totp(code),
            MfaProofInput::RecoveryCode(code) => Self::RecoveryCode(code),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct RecoveryCodesOutput {
    pub recovery_codes: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AcceptedOutput {
    pub accepted: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LoginOutput {
    Authenticated {
        session_id: Uuid,
        expires_at: DateTime<Utc>,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    MfaRequired {
        challenge: String,
    },
}

#[derive(Serialize, ToSchema)]
pub struct OidcRedirectOutput {
    pub authorization_url: String,
}
