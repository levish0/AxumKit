use application::config::SecretString;
use kit::config::{DatabaseSettings, HttpSettings, ObservabilitySettings, Settings};
use test_support::{TestDatabase, identity_settings};

pub fn settings(database: &TestDatabase) -> anyhow::Result<Settings> {
    Ok(Settings {
        database: DatabaseSettings {
            url: SecretString::new(database.url.clone()),
            max_connections: 8,
        },
        identity: identity_settings()?,
        http: HttpSettings {
            bind_address: "127.0.0.1:0".parse()?,
            allowed_origins: vec!["http://localhost:3000".into()],
            internal_proxy_secret: None,
            secure_cookies: false,
            timeout_seconds: 15,
            max_concurrent_requests: 32,
        },
        smtp: mail::SmtpSettings {
            host: "127.0.0.1".into(),
            port: 10259,
            starttls: false,
            certificate_authority: None,
            username: None,
            password: None,
            sender: "Test <noreply@example.test>".into(),
        },
        worker: jobs::WorkerSettings::default(),
        maintenance_interval_seconds: 300,
        retention: kit::config::RetentionSettings {
            succeeded_job_days: 30,
            failed_job_days: 90,
            audit_days: 180,
            api_usage_days: 90,
            notification_days: 90,
        },
        files: application::files::FileSettings::default(),
        post_search: None,
        profile_search: None,
        fcm: None,
        oidc: Vec::new(),
        human_verification: None,
        observability: ObservabilitySettings {
            log_filter: "info,sqlx=off,sea_orm=off,reqwest=off,hyper=off".into(),
            sentry_dsn: None,
            sentry_sample_rate: 0.0,
        },
    })
}
