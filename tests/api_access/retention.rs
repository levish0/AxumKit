use super::*;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn storage_failure_rolls_back_both_counters_and_usage_and_retention_preserves_current_counters()
-> Result<()> {
    let fixture = Fixture::create(10, 100).await?;
    let conflicting_id = Uuid::now_v7();
    api_usage::ActiveModel {
        id: Set(conflicting_id),
        client_id: Set(fixture.client_id),
        key_id: Set(fixture.key_id),
        operation: Set(application::api_access::ApiScope::OrganizationsRead),
        request_id: Set(Uuid::now_v7()),
        admitted_at: Set(database_now(&fixture.database).await?),
    }
    .insert(&fixture.database.connection)
    .await?;
    // A temporary uniqueness rule forces ledger insertion to fail after the counter update.
    fixture
        .database
        .connection
        .execute(
            &Index::create()
                .name("fixture_one_usage_per_client")
                .unique()
                .table(api_usage::Entity)
                .col(api_usage::Column::ClientId)
                .to_owned(),
        )
        .await?;
    let failed = fixture
        .app
        .api_access
        .read_organization(&fixture.credential, TraceContext::default())
        .await;
    assert!(matches!(failed, Err(ApplicationError::Storage(_))));
    fixture
        .database
        .connection
        .execute(
            &Index::drop()
                .name("fixture_one_usage_per_client")
                .table(api_usage::Entity)
                .to_owned(),
        )
        .await?;
    api_usage::Entity::delete_by_id(conflicting_id)
        .exec(&fixture.database.connection)
        .await?;
    let expired_usage_id = Uuid::now_v7();
    let current = api_client::Entity::find_by_id(fixture.client_id)
        .one(&fixture.database.connection)
        .await?
        .unwrap();
    assert_eq!((current.minute_used, current.month_used), (0, 0));
    assert_eq!(
        api_usage::Entity::find()
            .count(&fixture.database.connection)
            .await?,
        0
    );
    fixture
        .app
        .api_access
        .read_organization(&fixture.credential, TraceContext::default())
        .await?;
    api_usage::ActiveModel {
        id: Set(expired_usage_id),
        client_id: Set(fixture.client_id),
        key_id: Set(fixture.key_id),
        operation: Set(application::api_access::ApiScope::OrganizationsRead),
        request_id: Set(Uuid::now_v7()),
        admitted_at: Set(Utc::now() - chrono::Duration::days(91)),
    }
    .insert(&fixture.database.connection)
    .await?;
    assert_eq!(fixture.app.api_access.cleanup_usage(90).await?, 1);
    assert_eq!(
        api_usage::Entity::find()
            .count(&fixture.database.connection)
            .await?,
        1
    );
    let current = fixture
        .app
        .api_access
        .get_client(&fixture.actor, fixture.organization_id, fixture.client_id)
        .await?;
    assert_eq!(current.quota.month_used, 1);
    ensure!(
        fixture.app.api_access.cleanup_usage(0).await.is_err(),
        "invalid retention was accepted"
    );
    fixture.finish().await
}
