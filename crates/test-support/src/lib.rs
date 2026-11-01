use anyhow::{Context, Result, ensure};
use application::{
    authorization::Authorization,
    config::{IdentitySettings, SecretKeys, SecretString},
    identity::{AuthenticatedSession, IdentityService, SessionCredentials, entities::account},
};
use base64::Engine;
use jobs::TraceContext;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseBackend,
    DatabaseConnection, Set, Statement,
};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

pub mod storage;

pub const TEST_PASSWORD: &str = "test-only strong passphrase";

pub struct TestDatabase {
    pub connection: DatabaseConnection,
    pub url: String,
    administration: DatabaseConnection,
    name: String,
}

impl TestDatabase {
    pub async fn create() -> Result<Self> {
        let administration_url = std::env::var("TEST_DATABASE_URL")
            .context("TEST_DATABASE_URL is required; use just test-db")?;
        let mut parsed_url = url::Url::parse(&administration_url)?;
        ensure!(
            matches!(
                parsed_url.host_str(),
                Some("localhost" | "127.0.0.1" | "[::1]")
            ),
            "test database administration must be local"
        );
        let administration = connect(&administration_url).await?;
        let name = format!("kit_test_{}", Uuid::now_v7().simple());
        // SeaQuery has no CREATE/DROP DATABASE statements. The identifier is generated here.
        administration
            .execute_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("CREATE DATABASE \"{name}\""),
            ))
            .await?;
        parsed_url.set_path(&name);
        let url = parsed_url.to_string();
        let connection = connect(&url).await?;
        migration::migrate(&connection).await?;
        Ok(Self {
            connection,
            url,
            administration,
            name,
        })
    }

    pub async fn finish(self) -> Result<()> {
        self.connection.close().await?;
        self.administration
            .execute_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("DROP DATABASE \"{}\" WITH (FORCE)", self.name),
            ))
            .await?;
        self.administration.close().await?;
        Ok(())
    }

    pub async fn identity_service(&self) -> Result<IdentityService> {
        Ok(IdentityService::new(
            self.connection.clone(),
            identity_settings()?,
            Arc::new(Authorization::new()?),
        )
        .await?)
    }

    pub async fn verified_account(
        &self,
        identity: &IdentityService,
    ) -> Result<(AuthenticatedSession, SessionCredentials)> {
        let email = format!("{}@example.test", Uuid::now_v7());
        let password_hash =
            tokio::task::spawn_blocking(|| auth_core::password::hash(TEST_PASSWORD)).await??;
        account::ActiveModel {
            id: Set(Uuid::now_v7()),
            email: Set(email.clone()),
            password_hash: Set(Some(password_hash)),
            verified: Set(true),
            ..Default::default()
        }
        .insert(&self.connection)
        .await?;
        let credentials = match identity
            .login_with_password(&email, TEST_PASSWORD.into(), TraceContext::default())
            .await?
        {
            application::identity::LoginOutcome::Session(credentials) => credentials,
            _ => anyhow::bail!("fixture unexpectedly required MFA"),
        };
        let actor = identity.authenticate_session(&credentials.token).await?;
        Ok((actor, credentials))
    }

    pub async fn insert_challenge(
        &self,
        account_id: Uuid,
        purpose: application::identity::ChallengePurpose,
        token: &str,
    ) -> Result<()> {
        use application::identity::entities::challenge;
        use sea_orm::EntityTrait;
        let account = account::Entity::find_by_id(account_id)
            .one(&self.connection)
            .await?
            .context("fixture account missing")?;
        challenge::ActiveModel {
            id: Set(Uuid::now_v7()),
            account_id: Set(account_id),
            token_hash: Set(auth_core::token::hash_token(token)),
            purpose: Set(purpose),
            security_version: Set(account.security_version),
            expires_at: Set(chrono::Utc::now() + chrono::Duration::minutes(5)),
            ..Default::default()
        }
        .insert(&self.connection)
        .await?;
        Ok(())
    }
}

pub fn identity_settings() -> Result<IdentitySettings> {
    Ok(IdentitySettings {
        application_name: "Test Application".into(),
        public_url: url::Url::parse("http://localhost:3000/")?,
        secret_keys: SecretKeys::new(
            "test-v1".into(),
            [(
                "test-v1".into(),
                base64::engine::general_purpose::STANDARD.encode([17_u8; 32]),
            )]
            .into(),
        )?,
        rate_limit_key: SecretString::new("test-only-rate-limit-key-at-least-32-bytes"),
        password_concurrency: 2,
        authentication_identity_limit_per_minute: 10,
        authentication_client_limit_per_minute: 60,
        session_idle_seconds: 1800,
        session_absolute_seconds: 43_200,
    })
}

async fn connect(url: &str) -> Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(url);
    options
        .max_connections(8)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .sqlx_logging(false);
    Ok(Database::connect(options).await?)
}
