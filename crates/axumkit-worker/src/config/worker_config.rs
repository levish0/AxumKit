use dotenvy::dotenv;
use std::env;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
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

    // Frontend & Project
    pub frontend_host: String,
    pub project_name: String,
    pub frontend_path_verify_email: String,
    pub frontend_path_reset_password: String,
    pub frontend_path_confirm_email_change: String,

    // Database (Write only - worker does indexing, cleanup, etc.)
    pub db_write_host: String,
    pub db_write_port: String,
    pub db_write_name: String,
    pub db_write_user: String,
    pub db_write_password: String,
    pub db_write_max_connection: u32,
    pub db_write_min_connection: u32,

    // Cron
    pub cron_timezone: String,

    // SeaweedFS (revision content storage)
    pub seaweedfs_endpoint: String,

    // R2 (Sitemap storage)
    pub r2_endpoint: String,
    pub r2_region: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    pub r2_bucket_name: String,
    pub r2_public_domain: String,
}

static CONFIG: LazyLock<WorkerConfig> = LazyLock::new(|| {
    dotenv().ok();

    WorkerConfig {
        // SMTP
        smtp_host: env::var("SMTP_HOST").expect("SMTP_HOST must be set"),
        smtp_port: env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587),
        smtp_user: env::var("SMTP_USER").expect("SMTP_USER must be set"),
        smtp_password: env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD must be set"),
        smtp_tls: env::var("SMTP_TLS")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true),
        emails_from_email: env::var("EMAILS_FROM_EMAIL").expect("EMAILS_FROM_EMAIL must be set"),
        emails_from_name: env::var("EMAILS_FROM_NAME").unwrap_or_else(|_| "SevenWiki".into()),

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

        // Frontend & Project
        frontend_host: env::var("FRONTEND_HOST").expect("FRONTEND_HOST must be set"),
        project_name: env::var("PROJECT_NAME").expect("PROJECT_NAME must be set"),
        frontend_path_verify_email: env::var("FRONTEND_PATH_VERIFY_EMAIL")
            .expect("FRONTEND_PATH_VERIFY_EMAIL must be set"),
        frontend_path_reset_password: env::var("FRONTEND_PATH_RESET_PASSWORD")
            .expect("FRONTEND_PATH_RESET_PASSWORD must be set"),
        frontend_path_confirm_email_change: env::var("FRONTEND_PATH_CONFIRM_EMAIL_CHANGE")
            .expect("FRONTEND_PATH_CONFIRM_EMAIL_CHANGE must be set"),

        // Database (Write only)
        db_write_host: env::var("POSTGRES_WRITE_HOST")
            .expect("POSTGRES_WRITE_HOST must be set"),
        db_write_port: env::var("POSTGRES_WRITE_PORT")
            .expect("POSTGRES_WRITE_PORT must be set"),
        db_write_name: env::var("POSTGRES_WRITE_NAME")
            .expect("POSTGRES_WRITE_NAME must be set"),
        db_write_user: env::var("POSTGRES_WRITE_USER")
            .expect("POSTGRES_WRITE_USER must be set"),
        db_write_password: env::var("POSTGRES_WRITE_PASSWORD")
            .expect("POSTGRES_WRITE_PASSWORD must be set"),
        db_write_max_connection: env::var("POSTGRES_WRITE_MAX_CONNECTION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10),
        db_write_min_connection: env::var("POSTGRES_WRITE_MIN_CONNECTION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2),

        // Cron
        cron_timezone: env::var("CRON_TIMEZONE").unwrap_or_else(|_| "UTC".into()),

        // SeaweedFS
        seaweedfs_endpoint: env::var("SEAWEEDFS_ENDPOINT").expect("SEAWEEDFS_ENDPOINT must be set"),

        // R2
        r2_endpoint: env::var("R2_ENDPOINT").expect("R2_ENDPOINT must be set"),
        r2_region: env::var("R2_REGION").unwrap_or_else(|_| "auto".into()),
        r2_access_key_id: env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID must be set"),
        r2_secret_access_key: env::var("R2_SECRET_ACCESS_KEY")
            .expect("R2_SECRET_ACCESS_KEY must be set"),
        r2_bucket_name: env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set"),
        r2_public_domain: env::var("R2_PUBLIC_DOMAIN").expect("R2_PUBLIC_DOMAIN must be set"),
    }
});

impl WorkerConfig {
    pub fn get() -> &'static WorkerConfig {
        &CONFIG
    }

    pub fn redis_cache_url(&self) -> String {
        format!(
            "redis://{}:{}",
            self.redis_cache_host, self.redis_cache_port
        )
    }

    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.db_write_user,
            self.db_write_password,
            self.db_write_host,
            self.db_write_port,
            self.db_write_name
        )
    }
}
