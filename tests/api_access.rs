mod support;

use anyhow::{Context, Result, ensure};
use application::{
    ApplicationError,
    api_access::ApiScope,
    api_access::entities::{api_client, api_key, api_usage},
    identity::AuthenticatedSession,
};
use axum::http::{StatusCode, header};
use axum_test::TestServer;
use chrono::{DateTime, Datelike, Timelike, Utc};
use jobs::TraceContext;
use kit::bootstrap::{Application, bootstrap};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, EntityTrait, ExprTrait, IntoActiveModel, PaginatorTrait,
    QuerySelect, Set, TransactionTrait,
    sea_query::{Alias, Expr, Func, Index, LockType, Query},
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use test_support::TestDatabase;
use uuid::Uuid;

struct Fixture {
    database: TestDatabase,
    app: Application,
    actor: AuthenticatedSession,
    owner_token: String,
    organization_id: Uuid,
    client_id: Uuid,
    credential: String,
    key_id: Uuid,
}
impl Fixture {
    async fn create(minute: i64, month: i64) -> Result<Self> {
        let database = TestDatabase::create().await?;
        let app = bootstrap(&support::settings(&database)?).await?;
        let (actor, credentials) = database.verified_account(&app.identity).await?;
        let organization = app
            .organizations
            .create_organization(&actor, "Developer organization".into())
            .await?;
        let client = app
            .api_access
            .create_client(
                &actor,
                organization.id,
                "Partner application".into(),
                vec![ApiScope::OrganizationsRead],
                TraceContext::default(),
            )
            .await?;
        let issued = app
            .api_access
            .issue_key(
                &actor,
                organization.id,
                client.id,
                "Primary".into(),
                90,
                TraceContext::default(),
            )
            .await?;
        app.api_access
            .set_quota_by_operator(client.id, minute, month, TraceContext::default())
            .await?;
        Ok(Self {
            database,
            app,
            actor,
            owner_token: credentials.token,
            organization_id: organization.id,
            client_id: client.id,
            credential: issued.credential.expose().into(),
            key_id: issued.key.id,
        })
    }
    async fn finish(self) -> Result<()> {
        self.app.close().await?;
        self.database.finish().await
    }
}

async fn database_now(database: &TestDatabase) -> Result<DateTime<Utc>> {
    Ok(database
        .connection
        .query_one(
            &Query::select()
                .expr_as(Func::cust("clock_timestamp"), "now")
                .to_owned(),
        )
        .await?
        .context("database time missing")?
        .try_get("", "now")?)
}
async fn stable_minute(database: &TestDatabase) -> Result<()> {
    // Keep real-time counter assertions away from a legitimate fixed-window rollover.
    loop {
        let now = database_now(database).await?;
        if now.second() < 45 {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(61 - u64::from(now.second()))).await;
    }
}

#[path = "api_access/http_contracts.rs"]
mod http_contracts;
#[path = "api_access/management.rs"]
mod management;
#[path = "api_access/quota.rs"]
mod quota;
#[path = "api_access/retention.rs"]
mod retention;
