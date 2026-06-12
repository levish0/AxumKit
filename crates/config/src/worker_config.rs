use dotenvy::dotenv;
use std::env;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
/// Configuration model for worker config.
pub struct WorkerConfig {
    // SMTP
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub smtp_tls: bool,
    pub emails_from_email: String,
    pub emails_from_name: String,

    // MeiliSearch
    pub meilisearch_host: String,
    pub meilisearch_api_key: Option<String>,

    // NATS (Job Queue)
    pub nats_url: String,

    // Redis Cache (View counts, etc.)
    pub redis_cache_host: String,
    pub redis_cache_port: String,
    // Redis Lock (distributed locks; noeviction)
    pub redis_lock_host: String,
    pub redis_lock_port: String,

    // Image processor microservice
    pub image_processor_url: String,
    pub image_processor_timeout_secs: u64,

    // Frontend & Project
    pub frontend_host: String,
    pub project_name: String,
    pub frontend_path_verify_email: String,
    pub frontend_path_reset_password: String,
    pub frontend_path_confirm_email_change: String,

    // Database endpoint used by worker jobs.
    pub db_host: String,
    pub db_port: String,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,
    pub db_max_connection: u32,
    pub db_min_connection: u32,

    // Cron
    pub cron_timezone: String,

    // Cloudflare R2 (shared credentials)
    pub r2_endpoint: String,
    pub r2_region: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    // R2 Assets (public bucket - images, sitemap)
    pub r2_assets_bucket_name: String,
    pub r2_assets_public_domain: String,
}

static CONFIG: LazyLock<WorkerConfig> = LazyLock::new(|| {
    dotenv().ok();

    let mut errors: Vec<String> = Vec::new();

    macro_rules! require {
        ($name:expr) => {
            env::var($name).unwrap_or_else(|_| {
                errors.push(format!("  - {} (missing)", $name));
                String::new()
            })
        };
    }

    // Required string vars
    let smtp_host = require!("SMTP_HOST");
    let smtp_user = require!("SMTP_USER");
    let smtp_password = require!("SMTP_PASSWORD");
    let emails_from_email = require!("EMAILS_FROM_EMAIL");
    let frontend_host = require!("FRONTEND_HOST");
    let project_name = require!("PROJECT_NAME");
    let frontend_path_verify_email = require!("FRONTEND_PATH_VERIFY_EMAIL");
    let frontend_path_reset_password = require!("FRONTEND_PATH_RESET_PASSWORD");
    let frontend_path_confirm_email_change = require!("FRONTEND_PATH_CONFIRM_EMAIL_CHANGE");
    let db_host = require!("POSTGRES_HOST");
    let db_port = require!("POSTGRES_PORT");
    let db_name = require!("POSTGRES_NAME");
    let db_user = require!("POSTGRES_USER");
    let db_password = require!("POSTGRES_PASSWORD");
    let r2_endpoint = require!("R2_ENDPOINT");
    let r2_access_key_id = require!("R2_ACCESS_KEY_ID");
    let r2_secret_access_key = require!("R2_SECRET_ACCESS_KEY");
    let r2_assets_bucket_name = require!("R2_ASSETS_BUCKET_NAME");
    let r2_assets_public_domain = require!("R2_ASSETS_PUBLIC_DOMAIN");

    // Panic with all errors at once
    if !errors.is_empty() {
        panic!(
            "\n\nMissing required environment variables ({} errors):\n{}\n",
            errors.len(),
            errors.join("\n")
        );
    }

    WorkerConfig {
        // SMTP
        smtp_host,
        smtp_port: env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587),
        smtp_user,
        smtp_password,
        smtp_tls: env::var("SMTP_TLS")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true),
        emails_from_email,
        emails_from_name: env::var("EMAILS_FROM_NAME").unwrap_or_else(|_| "AxumKit".into()),

        // MeiliSearch
        meilisearch_host: env::var("MEILISEARCH_HOST")
            .unwrap_or_else(|_| "http://localhost:7700".into()),
        meilisearch_api_key: env::var("MEILISEARCH_API_KEY")
            .ok()
            .filter(|k| !k.is_empty()),

        // NATS (Job Queue)
        nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into()),

        // Redis Cache
        redis_cache_host: env::var("REDIS_CACHE_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
        redis_cache_port: env::var("REDIS_CACHE_PORT").unwrap_or_else(|_| "6380".into()),
        // Redis Lock
        redis_lock_host: env::var("REDIS_LOCK_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
        redis_lock_port: env::var("REDIS_LOCK_PORT").unwrap_or_else(|_| "6381".into()),

        // Image processor microservice
        image_processor_url: env::var("IMAGE_PROCESSOR_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:6701".into()),
        image_processor_timeout_secs: env::var("IMAGE_PROCESSOR_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),

        // Frontend & Project
        frontend_host,
        project_name,
        frontend_path_verify_email,
        frontend_path_reset_password,
        frontend_path_confirm_email_change,

        // Database
        db_host,
        db_port,
        db_name,
        db_user,
        db_password,
        db_max_connection: env::var("POSTGRES_MAX_CONNECTION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10),
        db_min_connection: env::var("POSTGRES_MIN_CONNECTION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2),

        // Cron
        cron_timezone: env::var("CRON_TIMEZONE").unwrap_or_else(|_| "UTC".into()),

        // Cloudflare R2 (shared credentials)
        r2_endpoint,
        r2_region: env::var("R2_REGION").unwrap_or_else(|_| "auto".into()),
        r2_access_key_id,
        r2_secret_access_key,
        // R2 Assets (public bucket)
        r2_assets_bucket_name,
        r2_assets_public_domain,
    }
});

impl WorkerConfig {
    /// Helper function for get.
    pub fn get() -> &'static WorkerConfig {
        &CONFIG
    }

    /// Helper function for redis cache url.
    pub fn redis_cache_url(&self) -> String {
        format!(
            "redis://{}:{}",
            self.redis_cache_host, self.redis_cache_port
        )
    }

    /// Helper function for redis lock url.
    pub fn redis_lock_url(&self) -> String {
        format!("redis://{}:{}", self.redis_lock_host, self.redis_lock_port)
    }

    /// Helper function for database url.
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.db_user, self.db_password, self.db_host, self.db_port, self.db_name
        )
    }
}
