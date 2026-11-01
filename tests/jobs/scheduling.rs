use super::*;
use sea_orm::{
    ConnectionTrait,
    sea_query::{Func, Query},
};

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn concurrent_schedulers_deduplicate_completed_ticks_and_isolate_queue_recovery()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let application = PostgresQueue::new(database.connection.clone());
    let maintenance = application.for_queue("maintenance")?;
    let now: chrono::DateTime<chrono::Utc> = database
        .connection
        .query_one(
            &Query::select()
                .expr_as(Func::cust("clock_timestamp"), "now")
                .to_owned(),
        )
        .await?
        .unwrap()
        .try_get("", "now")?;
    let timestamp = now.timestamp();
    let interval = [3600_i64, 3599, 3581, 86399]
        .into_iter()
        .find(|interval| {
            let elapsed = timestamp.rem_euclid(*interval);
            elapsed > 60 && elapsed < interval - 60
        })
        .expect("a stable test scheduling bucket");
    let (first, second) = tokio::join!(
        maintenance.schedule_periodic("system.maintenance", interval, TraceContext::default()),
        maintenance.schedule_periodic("system.maintenance", interval, TraceContext::default()),
    );
    assert_eq!(usize::from(first?) + usize::from(second?), 1);
    assert!(application.claim(10, 30).await?.is_none());
    let claimed = maintenance.claim(10, 30).await?.unwrap();
    job::Entity::update_many()
        .col_expr(
            job::Column::LeaseUntil,
            Expr::value(chrono::Utc::now() - chrono::Duration::seconds(1)),
        )
        .filter(job::Column::Id.eq(claimed.id))
        .exec(&database.connection)
        .await?;
    assert!(application.claim(10, 30).await?.is_none());
    assert!(maintenance.claim(10, 30).await?.is_none());
    job::Entity::update_many()
        .col_expr(
            job::Column::AvailableAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::seconds(1)),
        )
        .filter(job::Column::Id.eq(claimed.id))
        .exec(&database.connection)
        .await?;
    let replacement = maintenance.claim(10, 30).await?.unwrap();
    assert!(matches!(
        maintenance.finish(&claimed, None).await,
        Err(jobs::JobError::LeaseLost)
    ));
    maintenance.finish(&replacement, None).await?;
    assert!(
        !maintenance
            .schedule_periodic("system.maintenance", interval, TraceContext::default())
            .await?
    );
    let restarted = PostgresQueue::new(database.connection.clone()).for_queue("maintenance")?;
    assert!(
        !restarted
            .schedule_periodic("system.maintenance", interval, TraceContext::default())
            .await?
    );
    let scheduled = job::Entity::find_by_id(claimed.id)
        .one(&database.connection)
        .await?
        .unwrap();
    assert!(scheduled.schedule_key.is_some());
    assert_eq!(scheduled.queue, "maintenance");
    let application_id = queued_job(&database).await?;
    let application_claim = application.claim(10, 30).await?.unwrap();
    assert_eq!(application_claim.id, application_id);
    assert!(maintenance.claim(10, 30).await?.is_none());
    application.finish(&application_claim, None).await?;
    job::Entity::update_many()
        .col_expr(
            job::Column::FinishedAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::days(31)),
        )
        .exec(&database.connection)
        .await?;
    assert_eq!(maintenance.cleanup_finished(30, 90).await?, 2);
    database.finish().await
}
