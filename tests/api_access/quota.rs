use super::*;

async fn wait_for_client_lock(database: &TestDatabase, blocker: i32) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let waiting = database
                .connection
                .query_one(
                    &Query::select()
                        .expr(Expr::value(1))
                        .from(Alias::new("pg_stat_activity"))
                        .and_where(
                            Expr::col(Alias::new("datname")).eq(Func::cust("current_database")),
                        )
                        .and_where(Expr::col(Alias::new("wait_event_type")).eq("Lock"))
                        .and_where(
                            Expr::col(Alias::new("query"))
                                .like("SELECT%FROM \"api_clients\"%FOR NO KEY UPDATE%"),
                        )
                        .and_where(
                            Expr::expr(
                                Func::cust("array_position")
                                    .arg(
                                        Func::cust("pg_blocking_pids")
                                            .arg(Expr::col(Alias::new("pid"))),
                                    )
                                    .arg(blocker),
                            )
                            .is_not_null(),
                        )
                        .limit(1)
                        .to_owned(),
                )
                .await?;
            if waiting.is_some() {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .context("admission did not wait for the client lock")??;
    Ok(())
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn concurrent_instances_never_overallocate_the_remaining_client_quota() -> Result<()> {
    let fixture = Fixture::create(5, 5).await?;
    let second = bootstrap(&support::settings(&fixture.database)?).await?;
    stable_minute(&fixture.database).await?;
    let barrier = Arc::new(tokio::sync::Barrier::new(24));
    let request_id = Uuid::now_v7();
    let mut tasks = Vec::new();
    for index in 0..24 {
        let service = if index % 2 == 0 {
            fixture.app.api_access.clone()
        } else {
            second.api_access.clone()
        };
        let credential = fixture.credential.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            service
                .read_organization(&credential, TraceContext { request_id })
                .await
        }));
    }
    let mut accepted = 0;
    for task in tasks {
        match task.await? {
            Ok(_) => accepted += 1,
            Err(ApplicationError::ApiQuotaExceeded { .. }) => {}
            _ => anyhow::bail!("unexpected concurrent admission outcome"),
        }
    }
    assert_eq!(accepted, 5);
    let client = api_client::Entity::find_by_id(fixture.client_id)
        .one(&fixture.database.connection)
        .await?
        .unwrap();
    assert_eq!((client.minute_used, client.month_used), (5, 5));
    assert_eq!(
        api_usage::Entity::find()
            .count(&fixture.database.connection)
            .await?,
        5
    );
    second.close().await?;
    fixture.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn admission_rechecks_mutable_authority_after_waiting_for_the_client_lock() -> Result<()> {
    let fixture = Fixture::create(10, 100).await?;
    stable_minute(&fixture.database).await?;
    fixture
        .app
        .api_access
        .read_organization(&fixture.credential, TraceContext::default())
        .await?;
    for change in [
        "expiry",
        "key_revocation",
        "client_revocation",
        "scopes",
        "quota",
    ] {
        let transaction = fixture.database.connection.begin().await?;
        let blocker: i32 = transaction
            .query_one(
                &Query::select()
                    .expr_as(Func::cust("pg_backend_pid"), "pid")
                    .to_owned(),
            )
            .await?
            .unwrap()
            .try_get("", "pid")?;
        let client = api_client::Entity::find_by_id(fixture.client_id)
            .lock(LockType::NoKeyUpdate)
            .one(&transaction)
            .await?
            .unwrap();
        let mut waiting = tokio::spawn({
            let service = fixture.app.api_access.clone();
            let credential = fixture.credential.clone();
            async move {
                service
                    .read_organization(&credential, TraceContext::default())
                    .await
            }
        });
        let mutation: Result<()> = async {
            wait_for_client_lock(&fixture.database, blocker).await?;
            let key = api_key::Entity::find_by_id(fixture.key_id)
                .one(&transaction)
                .await?
                .unwrap();
            let now = database_now(&fixture.database).await?;
            let mut key = key.into_active_model();
            let mut client = client.into_active_model();
            key.expires_at = Set(now + chrono::Duration::days(1));
            key.revoked_at = Set(None);
            client.active = Set(true);
            client.scopes = Set(json!([ApiScope::OrganizationsRead]));
            client.month_limit = Set(100);
            match change {
                "expiry" => key.expires_at = Set(now - chrono::Duration::seconds(1)),
                "key_revocation" => key.revoked_at = Set(Some(now)),
                "client_revocation" => client.active = Set(false),
                "scopes" => client.scopes = Set(json!([])),
                "quota" => client.month_limit = Set(1),
                _ => unreachable!(),
            }
            key.update(&transaction).await?;
            client.update(&transaction).await?;
            transaction.commit().await?;
            Ok(())
        }
        .await;
        if let Err(error) = mutation {
            waiting.abort();
            let _ = waiting.await;
            return Err(error);
        }
        let result = tokio::time::timeout(Duration::from_secs(10), &mut waiting).await??;
        match change {
            "scopes" => assert!(matches!(result, Err(ApplicationError::Forbidden))),
            "quota" => assert!(matches!(
                result,
                Err(ApplicationError::ApiQuotaExceeded { .. })
            )),
            _ => assert!(matches!(result, Err(ApplicationError::Unauthenticated))),
        }
        assert_eq!(
            api_usage::Entity::find()
                .count(&fixture.database.connection)
                .await?,
            1
        );
        let client = api_client::Entity::find_by_id(fixture.client_id)
            .one(&fixture.database.connection)
            .await?
            .unwrap();
        assert_eq!((client.minute_used, client.month_used), (1, 1));
        // Restore authority before starting the next blocked request.
        let mut client = client.into_active_model();
        client.active = Set(true);
        client.scopes = Set(json!([ApiScope::OrganizationsRead]));
        client.month_limit = Set(100);
        client.update(&fixture.database.connection).await?;
        let mut key = api_key::Entity::find_by_id(fixture.key_id)
            .one(&fixture.database.connection)
            .await?
            .unwrap()
            .into_active_model();
        key.revoked_at = Set(None);
        key.expires_at = Set(database_now(&fixture.database).await? + chrono::Duration::days(1));
        key.update(&fixture.database.connection).await?;
    }
    fixture.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn invalid_secrets_do_not_wait_for_the_client_authority_lock() -> Result<()> {
    let fixture = Fixture::create(10, 100).await?;
    let transaction = fixture.database.connection.begin().await?;
    api_client::Entity::find_by_id(fixture.client_id)
        .lock(LockType::NoKeyUpdate)
        .one(&transaction)
        .await?;
    let wrong = format!("hak_v1_{}.{}", fixture.key_id.simple(), "A".repeat(43));
    assert_ne!(wrong, fixture.credential);
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        fixture
            .app
            .api_access
            .read_organization(&wrong, TraceContext::default()),
    )
    .await;
    transaction.rollback().await?;
    assert!(matches!(
        result.context("invalid secret waited for authority")?,
        Err(ApplicationError::Unauthenticated)
    ));
    fixture.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn persisted_minute_and_calendar_month_usage_roll_over_independently() -> Result<()> {
    let fixture = Fixture::create(10, 100).await?;
    stable_minute(&fixture.database).await?;
    let now = database_now(&fixture.database).await?;
    let minute = now.with_second(0).unwrap().with_nanosecond(0).unwrap();
    let month = minute
        .with_day(1)
        .unwrap()
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap();
    let mut client = api_client::Entity::find_by_id(fixture.client_id)
        .one(&fixture.database.connection)
        .await?
        .unwrap()
        .into_active_model();
    client.minute_started_at = Set(Some(minute - chrono::Duration::minutes(1)));
    client.minute_used = Set(10);
    client.month_started_at = Set(Some(month));
    client.month_used = Set(4);
    client.update(&fixture.database.connection).await?;
    let first = fixture
        .app
        .api_access
        .read_organization(&fixture.credential, TraceContext::default())
        .await?;
    assert_eq!((first.quota.minute_used, first.quota.month_used), (1, 5));
    let mut client = api_client::Entity::find_by_id(fixture.client_id)
        .one(&fixture.database.connection)
        .await?
        .unwrap()
        .into_active_model();
    client.minute_started_at = Set(Some(minute));
    client.minute_used = Set(3);
    client.month_started_at = Set(Some(month - chrono::Duration::days(1)));
    client.month_used = Set(100);
    client.update(&fixture.database.connection).await?;
    let second = fixture
        .app
        .api_access
        .read_organization(&fixture.credential, TraceContext::default())
        .await?;
    assert_eq!((second.quota.minute_used, second.quota.month_used), (4, 1));
    let client = api_client::Entity::find_by_id(fixture.client_id)
        .one(&fixture.database.connection)
        .await?
        .unwrap();
    assert_eq!(
        (client.minute_started_at, client.month_started_at),
        (Some(minute), Some(month))
    );
    fixture.finish().await
}
