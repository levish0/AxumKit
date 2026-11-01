use super::*;
use application::identity::entities::audit_event;
use jobs::{PostgresQueue, TraceContext, entities::job};
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};

#[tokio::test]
#[ignore = "requires PostgreSQL and Mailpit; run just test-process"]
async fn maintenance_replicas_schedule_cleanup_and_leave_application_jobs_for_workers() -> Result<()>
{
    let database = TestDatabase::create().await?;
    let mut environment = ProcessEnvironment::new(&database)?;
    let timestamp = chrono::Utc::now().timestamp();
    let interval = [3600_i64, 3599, 3581, 86399]
        .into_iter()
        .find(|interval| {
            let elapsed = timestamp.rem_euclid(*interval);
            elapsed > 60 && elapsed < interval - 60
        })
        .context("stable maintenance test interval missing")?;
    environment
        .values
        .insert("MAINTENANCE_INTERVAL_SECONDS".into(), interval.to_string());
    let expired_audit_id = Uuid::now_v7();
    audit_event::ActiveModel {
        id: Set(expired_audit_id),
        account_id: Set(None),
        event: Set("maintenance_fixture".into()),
        request_id: Set(Uuid::now_v7()),
        created_at: Set(chrono::Utc::now() - chrono::Duration::days(181)),
        ..Default::default()
    }
    .insert(&database.connection)
    .await?;
    let transaction = database.connection.begin().await?;
    let application_job_id = Uuid::now_v7();
    jobs::enqueue(
        &transaction,
        application_job_id,
        "test.application_job",
        &json!({}),
        TraceContext::default(),
    )
    .await?;
    transaction.commit().await?;
    let mut first = environment.start(env!("CARGO_BIN_EXE_maintenance"), None)?;
    let mut second = environment.start(env!("CARGO_BIN_EXE_maintenance"), None)?;
    let result = tokio::time::timeout(Duration::from_secs(25), async {
        loop {
            ensure!(
                first.child.try_wait()?.is_none() && second.child.try_wait()?.is_none(),
                "maintenance process exited"
            );
            let started = [&first, &second].iter().all(|process| {
                std::fs::read_to_string(process.log.path())
                    .is_ok_and(|log| log.contains("maintenance started"))
            });
            let completed = job::Entity::find()
                .filter(job::Column::Queue.eq("maintenance"))
                .filter(job::Column::State.eq(jobs::JobState::Succeeded))
                .count(&database.connection)
                .await?;
            if started && completed == 1 {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;
    first.stop().await?;
    second.stop().await?;
    result??;
    assert!(
        audit_event::Entity::find_by_id(expired_audit_id)
            .one(&database.connection)
            .await?
            .is_none()
    );
    let queue = PostgresQueue::new(database.connection.clone());
    let application_job = queue
        .claim(10, 30)
        .await?
        .context("application job was taken by maintenance")?;
    assert_eq!(application_job.id, application_job_id);
    queue.finish(&application_job, None).await?;
    assert_eq!(
        job::Entity::find()
            .filter(job::Column::Queue.eq("maintenance"))
            .count(&database.connection)
            .await?,
        1
    );
    for process in [&first, &second] {
        process.assert_secrets_absent(&[TEST_PASSWORD])?;
    }
    database.finish().await
}
