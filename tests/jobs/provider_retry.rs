use super::*;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn provider_retry_deadline_survives_finish_crash_recovery_and_stale_claims()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    let first_id = queued_job(&database).await?;
    let first = queue.claim(30, 120).await?.unwrap();
    queue.defer_retry(&first, 3600).await?;
    let first_deadline = job::Entity::find_by_id(first_id)
        .one(&database.connection)
        .await?
        .unwrap()
        .available_at;
    queue
        .finish(&first, Some(("provider_throttled", true)))
        .await?;
    assert!(
        job::Entity::find_by_id(first_id)
            .one(&database.connection)
            .await?
            .unwrap()
            .available_at
            >= first_deadline
    );
    assert!(queue.claim(30, 120).await?.is_none());
    let second_id = queued_job(&database).await?;
    let second = queue.claim(30, 120).await?.unwrap();
    assert_eq!(second.id, second_id);
    queue.defer_retry(&second, 7200).await?;
    let second_deadline = job::Entity::find_by_id(second_id)
        .one(&database.connection)
        .await?
        .unwrap()
        .available_at;
    job::Entity::update_many()
        .col_expr(
            job::Column::LeaseUntil,
            Expr::value(chrono::Utc::now() - chrono::Duration::seconds(1)),
        )
        .filter(job::Column::Id.eq(second.id))
        .exec(&database.connection)
        .await?;
    assert!(queue.claim(30, 120).await?.is_none());
    let recovered = job::Entity::find_by_id(second_id)
        .one(&database.connection)
        .await?
        .unwrap();
    assert_eq!(recovered.state, jobs::JobState::Pending);
    assert!(recovered.available_at >= second_deadline);
    assert!(matches!(
        queue.defer_retry(&second, 60).await,
        Err(jobs::JobError::LeaseLost)
    ));
    job::Entity::update_many()
        .col_expr(
            job::Column::AvailableAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::seconds(1)),
        )
        .filter(job::Column::Id.eq(second.id))
        .exec(&database.connection)
        .await?;
    let replacement = queue.claim(30, 120).await?.unwrap();
    assert_ne!(replacement.claim_token, second.claim_token);
    queue.finish(&replacement, None).await?;
    database.finish().await
}
