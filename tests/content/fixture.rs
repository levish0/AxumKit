use anyhow::{Context, Result};
use application::{
    config::SecretString,
    files::{S3Settings, SearchSettings},
    identity::{AuthenticatedSession, SessionCredentials},
    posts::{PostPublication, PostSummary, SavePost},
};
use axum_test::TestServer;
use jobs::TraceContext;
use kit::{
    bootstrap::{Application, bootstrap},
    config::Settings,
    http::build_router,
};
use serde_json::Value;
use test_support::TestDatabase;
use uuid::Uuid;

pub struct Fixture {
    pub database: TestDatabase,
    pub app: Application,
    pub settings: Settings,
    pub owner: AuthenticatedSession,
    pub owner_credentials: SessionCredentials,
    pub writer: AuthenticatedSession,
    pub writer_credentials: SessionCredentials,
    pub outsider: AuthenticatedSession,
    pub outsider_credentials: SessionCredentials,
    pub organization_id: Uuid,
}

impl Fixture {
    pub async fn new(integrations: bool, fcm: bool) -> Result<Self> {
        let database = TestDatabase::create().await?;
        let mut settings = crate::support::settings(&database)?;
        if integrations {
            let endpoint: url::Url = std::env::var("TEST_MEILISEARCH_URL")?.parse()?;
            let storage: url::Url = std::env::var("TEST_S3_ENDPOINT")?.parse()?;
            for url in [&endpoint, &storage] {
                anyhow::ensure!(
                    matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
                    "integration services must be local"
                );
            }
            let search = |name: &str| SearchSettings {
                endpoint: endpoint.clone(),
                api_key: SecretString::new("kit-local-meilisearch-key-development-only"),
                index: format!("{name}_{}", Uuid::now_v7().simple()),
                allow_http: true,
            };
            settings.files.search = Some(search("files"));
            settings.post_search = Some(search("posts"));
            settings.profile_search = Some(search("profiles"));
            settings.files.storage = Some(S3Settings {
                endpoint: storage,
                public_endpoint: None,
                bucket: format!("content-{}", Uuid::now_v7().simple()),
                region: "us-east-1".into(),
                access_key: SecretString::new("kit-local-access"),
                secret_key: SecretString::new("kit-local-development-secret-only"),
                allow_http: true,
            });
            settings.files.browser_origins = vec!["http://localhost:3000".into()];
        }
        if fcm {
            settings.fcm = Some(application::notifications::fcm::FcmSettings {
                project_id: "demo-project".into(),
            });
        }
        let app = bootstrap(&settings).await?;
        let (owner, owner_credentials) = database.verified_account(&app.identity).await?;
        let (writer, writer_credentials) = database.verified_account(&app.identity).await?;
        let (outsider, outsider_credentials) = database.verified_account(&app.identity).await?;
        let organization_id = app
            .organizations
            .create_organization(&owner, "Example organization".into())
            .await?
            .id;
        app.organizations
            .add_member(&owner, organization_id, writer.account_id())
            .await?;
        if integrations {
            app.files.initialize_integrations().await?;
            app.posts.initialize_search().await?;
            app.profiles.initialize_search().await?;
        }
        Ok(Self {
            database,
            app,
            settings,
            owner,
            owner_credentials,
            writer,
            writer_credentials,
            outsider,
            outsider_credentials,
            organization_id,
        })
    }

    pub fn server(&self) -> TestServer {
        TestServer::new(build_router(self.app.clone(), self.settings.http.clone()))
    }
    pub async fn post(
        &self,
        actor: &AuthenticatedSession,
        title: &str,
        state: PostPublication,
    ) -> Result<PostSummary> {
        Ok(self
            .app
            .posts
            .create_post(
                actor,
                self.organization_id,
                SavePost {
                    title: title.into(),
                    body: format!("{title} body"),
                    state,
                    attachment_ids: vec![],
                    expected_version: 0,
                },
                TraceContext::default(),
            )
            .await?)
    }
    pub async fn profile(&self, actor: &AuthenticatedSession, name: &str) -> Result<()> {
        self.app
            .profiles
            .save_profile(
                actor,
                application::profiles::SaveProfile {
                    display_name: name.into(),
                    bio: String::new(),
                    expected_version: 0,
                },
                TraceContext::default(),
            )
            .await?;
        Ok(())
    }
    pub async fn drain(&self) -> Result<()> {
        let mut completed = 0;
        while let Some(claim) = self.app.queue.claim(30, 120).await? {
            kit::job_handler::execute(&self.app, &claim)
                .await
                .map_err(|failure| anyhow::anyhow!("job failed: {}", failure.code))?;
            self.app.queue.finish(&claim, None).await?;
            completed += 1;
            anyhow::ensure!(completed < 200, "unexpected job loop");
        }
        Ok(())
    }
    pub async fn insert_search_document(
        &self,
        index: &SearchSettings,
        document: Value,
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let response: Value = client
            .post(
                index
                    .endpoint
                    .join(&format!("indexes/{}/documents", index.index))?,
            )
            .bearer_auth(index.api_key.expose())
            .json(&serde_json::json!([document]))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let task_id = response["taskUid"].as_u64().context("missing task ID")?;
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            loop {
                let status: Value = client
                    .get(index.endpoint.join(&format!("tasks/{task_id}"))?)
                    .bearer_auth(index.api_key.expose())
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                match status["status"].as_str() {
                    Some("succeeded") => return Ok::<(), anyhow::Error>(()),
                    Some("failed" | "canceled") => anyhow::bail!("index task failed"),
                    _ => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
                }
            }
        })
        .await??;
        Ok(())
    }
    pub async fn finish(self) -> Result<()> {
        self.app.close().await?;
        self.database.finish().await
    }
}
