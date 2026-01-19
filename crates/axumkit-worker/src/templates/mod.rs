//! Email template rendering using MJML and minijinja

use minijinja::{Environment, context};
use mrml::prelude::render::RenderOptions;
use std::sync::LazyLock;

/// MJML templates embedded at compile time
const EMAIL_VERIFICATION_MJML: &str = include_str!("email_verification.mjml");
const RESET_PASSWORD_MJML: &str = include_str!("reset_password.mjml");
const EMAIL_CHANGE_MJML: &str = include_str!("email_change.mjml");

/// Pre-rendered HTML templates (MJML → HTML conversion cached)
static EMAIL_VERIFICATION_HTML: LazyLock<String> = LazyLock::new(|| {
    mjml_to_html(EMAIL_VERIFICATION_MJML).expect("Failed to parse email verification template")
});

static RESET_PASSWORD_HTML: LazyLock<String> = LazyLock::new(|| {
    mjml_to_html(RESET_PASSWORD_MJML).expect("Failed to parse reset password template")
});

static EMAIL_CHANGE_HTML: LazyLock<String> = LazyLock::new(|| {
    mjml_to_html(EMAIL_CHANGE_MJML).expect("Failed to parse email change template")
});

/// Render email verification template
pub fn render_email_verification(
    project_name: &str,
    username: &str,
    verification_link: &str,
    valid_minutes: u64,
) -> Result<String, TemplateError> {
    render_with_context(
        &EMAIL_VERIFICATION_HTML,
        context! {
            project_name => project_name,
            username => username,
            verification_link => verification_link,
            valid_minutes => valid_minutes,
        },
    )
}

/// Render password reset template
pub fn render_password_reset(
    project_name: &str,
    username: &str,
    link: &str,
    valid_minutes: u64,
) -> Result<String, TemplateError> {
    render_with_context(
        &RESET_PASSWORD_HTML,
        context! {
            project_name => project_name,
            username => username,
            link => link,
            valid_minutes => valid_minutes,
        },
    )
}

/// Render email change confirmation template
pub fn render_email_change(
    project_name: &str,
    username: &str,
    confirmation_link: &str,
    valid_minutes: u64,
) -> Result<String, TemplateError> {
    render_with_context(
        &EMAIL_CHANGE_HTML,
        context! {
            project_name => project_name,
            username => username,
            confirmation_link => confirmation_link,
            valid_minutes => valid_minutes,
        },
    )
}

/// Convert MJML to HTML
fn mjml_to_html(mjml: &str) -> Result<String, TemplateError> {
    let root = mrml::parse(mjml).map_err(|e| TemplateError::MjmlParse(e.to_string()))?;
    let opts = RenderOptions::default();
    root.element
        .render(&opts)
        .map_err(|e| TemplateError::MjmlRender(e.to_string()))
}

/// Render HTML template with minijinja context
fn render_with_context(html: &str, ctx: minijinja::Value) -> Result<String, TemplateError> {
    let mut env = Environment::new();
    env.add_template("email", html)
        .map_err(|e| TemplateError::Jinja(e.to_string()))?;
    let template = env
        .get_template("email")
        .map_err(|e| TemplateError::Jinja(e.to_string()))?;
    template
        .render(ctx)
        .map_err(|e| TemplateError::Jinja(e.to_string()))
}

#[derive(Debug)]
pub enum TemplateError {
    MjmlParse(String),
    MjmlRender(String),
    Jinja(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MjmlParse(e) => write!(f, "MJML parse error: {}", e),
            Self::MjmlRender(e) => write!(f, "MJML render error: {}", e),
            Self::Jinja(e) => write!(f, "Template render error: {}", e),
        }
    }
}

impl std::error::Error for TemplateError {}
