use anyhow::{Context, Result, ensure};
use application::{
    config::{IdentitySettings, SecretKeys, SecretString},
    files::{FileSettings, S3Settings},
    human_verification::TurnstileSettings,
    identity::oidc::ProviderSettings,
    notifications::fcm::FcmSettings,
    search::SearchSettings,
};
use jobs::WorkerSettings;
use mail::SmtpSettings;
use std::{collections::BTreeMap, net::SocketAddr, str::FromStr};
use url::Url;

mod environment;
mod oidc;
mod proxy;
mod turnstile;

#[derive(Clone, Debug)]
pub struct DatabaseSettings {
    pub url: SecretString,
    pub max_connections: u32,
}

#[derive(Clone, Debug)]
pub struct HttpSettings {
    pub bind_address: SocketAddr,
    pub allowed_origins: Vec<String>,
    pub internal_proxy_secret: Option<SecretString>,
    pub secure_cookies: bool,
    pub timeout_seconds: u64,
    pub max_concurrent_requests: usize,
}
impl HttpSettings {
    pub fn session_cookie_name(&self) -> &'static str {
        if self.secure_cookies {
            "__Host-session"
        } else {
            "session"
        }
    }
    pub fn oauth_cookie_name(&self) -> &'static str {
        if self.secure_cookies {
            "__Host-oidc"
        } else {
            "oidc"
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObservabilitySettings {
    pub log_filter: String,
    pub sentry_dsn: Option<SecretString>,
    pub sentry_sample_rate: f32,
}

#[derive(Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub identity: IdentitySettings,
    pub http: HttpSettings,
    pub smtp: SmtpSettings,
    pub worker: WorkerSettings,
    pub files: FileSettings,
    pub post_search: Option<SearchSettings>,
    pub profile_search: Option<SearchSettings>,
    pub fcm: Option<FcmSettings>,
    pub oidc: Vec<ProviderSettings>,
    pub human_verification: Option<TurnstileSettings>,
    pub observability: ObservabilitySettings,
    pub retention: RetentionSettings,
    pub maintenance_interval_seconds: i64,
}

#[derive(Clone, Debug)]
pub struct RetentionSettings {
    pub succeeded_job_days: i64,
    pub failed_job_days: i64,
    pub audit_days: i64,
    pub api_usage_days: i64,
    pub notification_days: i64,
}

impl Settings {
    pub fn from_environment() -> Result<Self> {
        Self::from_values(&std::env::vars().collect())
    }

    pub fn from_values(values: &BTreeMap<String, String>) -> Result<Self> {
        let get = |name: &str, fallback: &str| {
            values.get(name).cloned().unwrap_or_else(|| fallback.into())
        };
        let required = |name: &str| {
            values
                .get(name)
                .filter(|value| !value.is_empty())
                .cloned()
                .with_context(|| format!("{name} is required"))
        };
        let environment: environment::ApplicationEnvironment =
            get("APP_ENV", "development").parse()?;
        let production = environment == environment::ApplicationEnvironment::Production;
        let public_url = Url::parse(&get("PUBLIC_URL", "http://localhost:3000/"))
            .context("invalid PUBLIC_URL")?;
        validate_web_url(&public_url, production)?;
        let allowed_origins: Vec<String> = get(
            "ALLOWED_ORIGINS",
            &public_url.origin().ascii_serialization(),
        )
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect();
        for origin in &allowed_origins {
            let url = Url::parse(origin).context("invalid ALLOWED_ORIGINS")?;
            validate_web_url(&url, production)?;
            ensure!(
                url.origin().ascii_serialization() == *origin,
                "ALLOWED_ORIGINS must contain exact origins without paths"
            );
        }
        let encoded_keys = serde_json::from_str(&required("SECRET_KEYS")?)
            .map_err(|_| anyhow::anyhow!("invalid SECRET_KEYS JSON map"))?;
        let identity = IdentitySettings {
            application_name: get("APP_NAME", "Hirune Axum Kit"),
            public_url,
            secret_keys: SecretKeys::new(required("ACTIVE_SECRET_KEY")?, encoded_keys)?,
            rate_limit_key: SecretString::new(required("RATE_LIMIT_KEY")?),
            password_concurrency: number(values, "PASSWORD_CONCURRENCY", 4)?,
            authentication_identity_limit_per_minute: number(
                values,
                "AUTH_IDENTITY_LIMIT_PER_MINUTE",
                10,
            )?,
            authentication_client_limit_per_minute: number(
                values,
                "AUTH_CLIENT_LIMIT_PER_MINUTE",
                60,
            )?,
            session_idle_seconds: number(values, "SESSION_IDLE_SECONDS", 1_209_600)?,
            session_absolute_seconds: number(values, "SESSION_ABSOLUTE_SECONDS", 7_776_000)?,
        };
        identity.validate()?;
        let database_url = required("DATABASE_URL")?;
        let parsed_database =
            Url::parse(&database_url).map_err(|_| anyhow::anyhow!("invalid DATABASE_URL"))?;
        ensure!(
            ["postgres", "postgresql"].contains(&parsed_database.scheme()),
            "DATABASE_URL must use PostgreSQL"
        );
        let database = DatabaseSettings {
            url: SecretString::new(database_url),
            max_connections: number(values, "DATABASE_MAX_CONNECTIONS", 10)?,
        };
        ensure!(
            (1..=100).contains(&database.max_connections),
            "DATABASE_MAX_CONNECTIONS must be between 1 and 100"
        );
        let http = HttpSettings {
            bind_address: number(
                values,
                "BIND_ADDRESS",
                SocketAddr::from(([127, 0, 0, 1], 3000)),
            )?,
            allowed_origins,
            internal_proxy_secret: proxy::parse_internal_proxy_secret(values)?,
            secure_cookies: production,
            timeout_seconds: number(values, "HTTP_TIMEOUT_SECONDS", 15)?,
            max_concurrent_requests: number(values, "HTTP_MAX_CONCURRENT_REQUESTS", 128)?,
        };
        ensure!(
            (1..=60).contains(&http.timeout_seconds)
                && (1..=4096).contains(&http.max_concurrent_requests),
            "invalid HTTP limits"
        );
        let smtp = SmtpSettings {
            host: get("SMTP_HOST", "localhost"),
            port: number(values, "SMTP_PORT", 1025)?,
            starttls: number(values, "SMTP_STARTTLS", false)?,
            certificate_authority: values
                .get("SMTP_CA_CERTIFICATE")
                .filter(|value| !value.is_empty())
                .map(std::path::PathBuf::from),
            username: values
                .get("SMTP_USERNAME")
                .filter(|value| !value.is_empty())
                .cloned(),
            password: values
                .get("SMTP_PASSWORD")
                .filter(|value| !value.is_empty())
                .cloned(),
            sender: get("MAIL_FROM", "Hirune <noreply@example.test>"),
        };
        ensure!(
            !production || smtp.starttls,
            "production SMTP requires STARTTLS"
        );
        let worker = WorkerSettings {
            concurrency: number(values, "WORKER_CONCURRENCY", 4)?,
            lease_seconds: number(values, "JOB_LEASE_SECONDS", 30)?,
            execution_seconds: number(values, "JOB_EXECUTION_SECONDS", 120)?,
            drain_seconds: number(values, "WORKER_DRAIN_SECONDS", 35)?,
            poll_milliseconds: number(values, "WORKER_POLL_MILLISECONDS", 250)?,
        };
        worker.validate()?;
        let supplied = |name: &str| values.get(name).is_some_and(|value| !value.is_empty());
        let storage = if [
            "S3_ENDPOINT",
            "S3_PUBLIC_ENDPOINT",
            "S3_BUCKET",
            "S3_ACCESS_KEY",
            "S3_SECRET_KEY",
        ]
        .iter()
        .any(|name| supplied(name))
        {
            Some(S3Settings {
                endpoint: Url::parse(&required("S3_ENDPOINT")?).context("invalid S3_ENDPOINT")?,
                public_endpoint: values
                    .get("S3_PUBLIC_ENDPOINT")
                    .filter(|value| !value.is_empty())
                    .map(|value| Url::parse(value).context("invalid S3_PUBLIC_ENDPOINT"))
                    .transpose()?,
                bucket: required("S3_BUCKET")?,
                region: get("S3_REGION", "us-east-1"),
                access_key: SecretString::new(required("S3_ACCESS_KEY")?),
                secret_key: SecretString::new(required("S3_SECRET_KEY")?),
                allow_http: number(values, "S3_ALLOW_HTTP", false)?,
            })
        } else {
            None
        };
        let search = if ["MEILISEARCH_URL", "MEILISEARCH_API_KEY"]
            .iter()
            .any(|name| supplied(name))
        {
            Some(SearchSettings {
                endpoint: Url::parse(&required("MEILISEARCH_URL")?)
                    .context("invalid MEILISEARCH_URL")?,
                api_key: SecretString::new(required("MEILISEARCH_API_KEY")?),
                index: get("MEILISEARCH_INDEX", "files"),
                allow_http: number(values, "MEILISEARCH_ALLOW_HTTP", false)?,
            })
        } else {
            None
        };
        let defaults = FileSettings::default();
        let post_search = search.clone().map(|mut search| {
            search.index = get("MEILISEARCH_POSTS_INDEX", "posts");
            search
        });
        let profile_search = search.clone().map(|mut search| {
            search.index = get("MEILISEARCH_PROFILES_INDEX", "profiles");
            search
        });
        if let (Some(files), Some(posts), Some(profiles)) = (&search, &post_search, &profile_search)
        {
            posts.validate()?;
            profiles.validate()?;
            ensure!(
                files.index != posts.index
                    && files.index != profiles.index
                    && posts.index != profiles.index,
                "search indexes must have distinct names"
            );
        }
        ensure!(
            search.is_some()
                || !["MEILISEARCH_POSTS_INDEX", "MEILISEARCH_PROFILES_INDEX"]
                    .iter()
                    .any(|name| supplied(name)),
            "search index names require Meilisearch credentials"
        );
        let fcm = if number(values, "FCM_ENABLED", false)? {
            let fcm = FcmSettings {
                project_id: required("FCM_PROJECT_ID")?,
            };
            fcm.validate()?;
            Some(fcm)
        } else {
            None
        };
        let files = FileSettings {
            storage,
            search,
            max_file_bytes: number(values, "FILE_MAX_BYTES", defaults.max_file_bytes)?,
            organization_quota_bytes: number(
                values,
                "FILE_ORGANIZATION_QUOTA_BYTES",
                defaults.organization_quota_bytes,
            )?,
            upload_url_seconds: number(
                values,
                "FILE_UPLOAD_URL_SECONDS",
                defaults.upload_url_seconds,
            )?,
            download_url_seconds: number(
                values,
                "FILE_DOWNLOAD_URL_SECONDS",
                defaults.download_url_seconds,
            )?,
            upload_grace_seconds: number(
                values,
                "FILE_UPLOAD_GRACE_SECONDS",
                defaults.upload_grace_seconds,
            )?,
            browser_origins: http.allowed_origins.clone(),
            organization_file_limit: number(
                values,
                "FILE_ORGANIZATION_LIMIT",
                defaults.organization_file_limit,
            )?,
            pending_file_limit: number(values, "FILE_PENDING_LIMIT", defaults.pending_file_limit)?,
            reservations_per_minute: number(
                values,
                "FILE_RESERVATIONS_PER_MINUTE",
                defaults.reservations_per_minute,
            )?,
            tombstone_days: number(values, "FILE_TOMBSTONE_DAYS", defaults.tombstone_days)?,
        };
        files.validate()?;
        let oidc = oidc::parse_providers(values, production)?;
        let human_verification = turnstile::parse_turnstile(values, production)?;
        let observability = ObservabilitySettings {
            log_filter: get(
                "RUST_LOG",
                "info,sqlx=off,sea_orm=off,reqwest=off,hyper=off",
            ),
            sentry_dsn: values
                .get("SENTRY_DSN")
                .filter(|value| !value.is_empty())
                .cloned()
                .map(SecretString::new),
            sentry_sample_rate: number(values, "SENTRY_SAMPLE_RATE", 0.05)?,
        };
        ensure!(
            (0.0..=1.0).contains(&observability.sentry_sample_rate),
            "invalid SENTRY_SAMPLE_RATE"
        );
        let retention = RetentionSettings {
            succeeded_job_days: number(values, "SUCCEEDED_JOB_RETENTION_DAYS", 30)?,
            failed_job_days: number(values, "FAILED_JOB_RETENTION_DAYS", 90)?,
            audit_days: number(values, "AUDIT_RETENTION_DAYS", 180)?,
            api_usage_days: number(values, "API_USAGE_RETENTION_DAYS", 90)?,
            notification_days: number(values, "NOTIFICATION_RETENTION_DAYS", 90)?,
        };
        ensure!(
            [
                retention.succeeded_job_days,
                retention.failed_job_days,
                retention.audit_days,
                retention.api_usage_days,
                retention.notification_days
            ]
            .into_iter()
            .all(|days| (1..=36500).contains(&days)),
            "retention must be between 1 and 36500 days"
        );
        let maintenance_interval_seconds = number(values, "MAINTENANCE_INTERVAL_SECONDS", 300)?;
        ensure!(
            (1..=86400).contains(&maintenance_interval_seconds),
            "MAINTENANCE_INTERVAL_SECONDS must be between 1 and 86400"
        );
        Ok(Self {
            database,
            identity,
            http,
            smtp,
            worker,
            files,
            post_search,
            profile_search,
            fcm,
            oidc,
            human_verification,
            observability,
            retention,
            maintenance_interval_seconds,
        })
    }
}

fn number<T: FromStr>(values: &BTreeMap<String, String>, name: &str, fallback: T) -> Result<T> {
    match values.get(name) {
        Some(value) => value.parse().map_err(|_| anyhow::anyhow!("invalid {name}")),
        None => Ok(fallback),
    }
}

fn validate_web_url(url: &Url, production: bool) -> Result<()> {
    ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "web URLs cannot contain credentials, queries or fragments"
    );
    ensure!(
        url.scheme() == "https"
            || (!production
                && url.scheme() == "http"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))),
        "HTTPS is required outside local development"
    );
    Ok(())
}
