use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart},
    transport::smtp::{
        authentication::Credentials,
        client::{Certificate, Tls, TlsParameters},
    },
};
use minijinja::Environment;
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf, sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(Clone)]
pub struct SmtpSettings {
    pub host: String,
    pub port: u16,
    pub starttls: bool,
    pub certificate_authority: Option<PathBuf>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub sender: String,
}
impl fmt::Debug for SmtpSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SmtpSettings")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("starttls", &self.starttls)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthenticationMail {
    pub recipient: String,
    pub application_name: String,
    pub purpose: MailPurpose,
    pub action_url: String,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailPurpose {
    VerifyEmail,
    ResetPassword,
    Welcome,
    PasswordChanged,
}
impl MailPurpose {
    pub fn heading(self) -> &'static str {
        match self {
            Self::VerifyEmail => "Verify your email",
            Self::ResetPassword => "Reset your password",
            Self::Welcome => "Welcome",
            Self::PasswordChanged => "Your password was changed",
        }
    }

    fn template_name(self) -> &'static str {
        match self {
            Self::VerifyEmail => "verify-email",
            Self::ResetPassword => "reset-password",
            Self::Welcome => "welcome",
            Self::PasswordChanged => "password-changed",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("mail configuration or rendering failed")]
    Template(String),
    #[error("mail message is invalid")]
    Message(#[from] lettre::error::Error),
    #[error("mailbox is invalid")]
    Address(#[from] lettre::address::AddressError),
    #[error("SMTP transport failed")]
    Transport(#[from] lettre::transport::smtp::Error),
}
impl MailError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Transport(error) if !error.is_permanent())
    }
}

#[derive(Clone)]
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    templates: Arc<Environment<'static>>,
    sender: Mailbox,
    message_id_domain: String,
}
pub struct SmtpAccepted;

pub fn valid_recipient(email: &str) -> bool {
    email.parse::<lettre::Address>().is_ok()
}

impl Mailer {
    pub fn new(settings: &SmtpSettings) -> Result<Self, MailError> {
        let sender: Mailbox = settings.sender.parse()?;
        let transport = if settings.starttls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.host)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.host)
        };
        let mut transport = transport
            .port(settings.port)
            .timeout(Some(Duration::from_secs(15)));
        if let Some(path) = &settings.certificate_authority {
            if !settings.starttls {
                return Err(MailError::Template(
                    "SMTP CA certificate requires STARTTLS".into(),
                ));
            }
            let certificate = std::fs::read(path)
                .map_err(|_| MailError::Template("SMTP CA certificate could not be read".into()))?;
            let parameters = TlsParameters::builder(settings.host.clone())
                .add_root_certificate(Certificate::from_pem(&certificate)?)
                .build()?;
            transport = transport.tls(Tls::Required(parameters));
        }
        match (&settings.username, &settings.password) {
            (Some(username), Some(password)) => {
                transport =
                    transport.credentials(Credentials::new(username.clone(), password.clone()));
            }
            (None, None) => {}
            _ => {
                return Err(MailError::Template(
                    "SMTP username and password must be set together".into(),
                ));
            }
        }
        let mut templates = Environment::new();
        templates.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
        for (name, mjml, text) in [
            (
                "verify-email",
                include_str!("../templates/verify-email.mjml"),
                include_str!("../templates/verify-email.txt"),
            ),
            (
                "reset-password",
                include_str!("../templates/reset-password.mjml"),
                include_str!("../templates/reset-password.txt"),
            ),
            (
                "welcome",
                include_str!("../templates/welcome.mjml"),
                include_str!("../templates/welcome.txt"),
            ),
            (
                "password-changed",
                include_str!("../templates/password-changed.mjml"),
                include_str!("../templates/password-changed.txt"),
            ),
        ] {
            let html = mrml::parse(mjml)
                .map_err(|error| MailError::Template(error.to_string()))?
                .element
                .render(&Default::default())
                .map_err(|error| MailError::Template(error.to_string()))?;
            templates
                .add_template_owned(format!("{name}.html"), html)
                .map_err(|error| MailError::Template(error.to_string()))?;
            templates
                .add_template_owned(format!("{name}.txt"), text.to_owned())
                .map_err(|error| MailError::Template(error.to_string()))?;
        }
        Ok(Self {
            message_id_domain: sender.email.domain().into(),
            sender,
            transport: transport.build(),
            templates: Arc::new(templates),
        })
    }

    pub fn render(
        &self,
        delivery_id: Uuid,
        mail: &AuthenticationMail,
    ) -> Result<Message, MailError> {
        let context = minijinja::context! {
            application_name => &mail.application_name, heading => mail.purpose.heading(), action_url => &mail.action_url,
        };
        let render = |name: &str| -> Result<String, MailError> {
            self.templates
                .get_template(name)
                .and_then(|template| template.render(&context))
                .map_err(|error| MailError::Template(error.to_string()))
        };
        Ok(Message::builder()
            .from(self.sender.clone())
            .to(mail.recipient.parse()?)
            .subject(format!(
                "{} — {}",
                mail.application_name,
                mail.purpose.heading()
            ))
            .message_id(Some(format!("<{delivery_id}@{}>", self.message_id_domain)))
            .multipart(MultiPart::alternative_plain_html(
                render(&format!("{}.txt", mail.purpose.template_name()))?,
                render(&format!("{}.html", mail.purpose.template_name()))?,
            ))?)
    }

    #[tracing::instrument(skip_all, fields(delivery_id = %delivery_id))]
    pub async fn send(
        &self,
        delivery_id: Uuid,
        mail: &AuthenticationMail,
    ) -> Result<SmtpAccepted, MailError> {
        let message = self.render(delivery_id, mail)?;
        self.transport.send(message).await?;
        tracing::info!(outcome = "smtp_accepted", "mail accepted by SMTP server");
        Ok(SmtpAccepted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn rendering_escapes_html_and_keeps_a_stable_multipart_message_id() {
        let mailer = Mailer::new(&SmtpSettings {
            host: "localhost".into(),
            port: 1025,
            starttls: false,
            certificate_authority: None,
            username: None,
            password: None,
            sender: "Test <noreply@example.test>".into(),
        })
        .unwrap();
        let delivery_id = Uuid::now_v7();
        let message = AuthenticationMail {
            recipient: "recipient@example.test".into(),
            application_name: "<script>test</script>".into(),
            purpose: MailPurpose::VerifyEmail,
            action_url: "https://example.test/verify#token=public-test-token".into(),
        };
        let html = mailer
            .templates
            .get_template("verify-email.html")
            .unwrap()
            .render(minijinja::context! {
                application_name => &message.application_name,
                heading => "Verify",
                action_url => &message.action_url,
            })
            .unwrap();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        for purpose in [
            MailPurpose::VerifyEmail,
            MailPurpose::ResetPassword,
            MailPurpose::Welcome,
            MailPurpose::PasswordChanged,
        ] {
            let message = AuthenticationMail {
                purpose,
                ..message.clone()
            };
            let rendered =
                String::from_utf8(mailer.render(delivery_id, &message).unwrap().formatted())
                    .unwrap();
            assert!(rendered.contains(&format!("Message-ID: <{delivery_id}@example.test>")));
            assert!(
                rendered.contains("multipart/alternative")
                    && rendered.contains("text/plain")
                    && rendered.contains("text/html")
            );
            let html = mailer
                .templates
                .get_template(&format!("{}.html", purpose.template_name()))
                .unwrap()
                .render(
                    minijinja::context! { application_name=>message.application_name,
                    heading=>purpose.heading(), action_url=>message.action_url },
                )
                .unwrap();
            assert!(!html.contains("<script>"));
            assert!(html.contains("&lt;script&gt;"));
            assert_eq!(
                html.contains("30 minutes"),
                matches!(
                    purpose,
                    MailPurpose::VerifyEmail | MailPurpose::ResetPassword
                )
            );
        }
    }
}
