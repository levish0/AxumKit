use crate::Mailer;
use crate::config::WorkerConfig;
use anyhow::Result;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use std::sync::Arc;

pub fn create_mailer(config: &WorkerConfig) -> Result<Mailer> {
    let creds = Credentials::new(config.smtp_user.clone(), config.smtp_password.clone());

    let transport = if config.smtp_tls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)?
            .port(config.smtp_port)
            .credentials(creds)
            .build()
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
            .port(config.smtp_port)
            .credentials(creds)
            .build()
    };

    Ok(Arc::new(transport))
}
