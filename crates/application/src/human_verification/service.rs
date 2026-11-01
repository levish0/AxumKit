use super::{HumanVerificationAction, TurnstileSettings, turnstile::TurnstileClient};
use crate::{ApplicationError, ApplicationResult};
use std::{net::IpAddr, sync::Arc};

/// Cloudflare issues Turnstile tokens of at most 2048 characters.
pub const MAX_HUMAN_VERIFICATION_TOKEN_BYTES: usize = 2048;

#[derive(Clone)]
pub struct HumanVerificationService {
    turnstile: Option<Arc<TurnstileClient>>,
}

impl HumanVerificationService {
    pub fn new(settings: Option<TurnstileSettings>) -> ApplicationResult<Self> {
        Ok(Self {
            turnstile: settings
                .map(TurnstileClient::new)
                .transpose()?
                .map(Arc::new),
        })
    }

    /// Accepts every request when Turnstile is not configured. Otherwise the
    /// token must pass siteverify for this action before the operation runs.
    #[tracing::instrument(skip_all, fields(action = action.as_str()))]
    pub async fn verify(
        &self,
        action: HumanVerificationAction,
        token: Option<&str>,
        remote_ip: Option<IpAddr>,
    ) -> ApplicationResult<()> {
        let Some(turnstile) = &self.turnstile else {
            return Ok(());
        };
        let token = token
            .filter(|token| !token.is_empty())
            .ok_or(ApplicationError::HumanVerificationFailed)?;
        if token.len() > MAX_HUMAN_VERIFICATION_TOKEN_BYTES {
            return Err(ApplicationError::InvalidInput(
                "human verification token is too long",
            ));
        }
        turnstile.verify(action, token, remote_ip).await
    }
}
