use super::*;
use sea_orm::{
    ConnectionTrait, ExprTrait, QuerySelect,
    sea_query::{Alias, Func, Query},
};

async fn wait_for_delete_lock(database: &TestDatabase) -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let blocked = database
                .connection
                .query_one(
                    &Query::select()
                        .expr(Expr::value(1))
                        .from(Alias::new("pg_stat_activity"))
                        .and_where(
                            Expr::col(Alias::new("datname")).eq(Func::cust("current_database")),
                        )
                        .and_where(Expr::col(Alias::new("wait_event_type")).eq("Lock"))
                        .and_where(Expr::col(Alias::new("state")).eq("active"))
                        .limit(1)
                        .to_owned(),
                )
                .await?;
            if blocked.is_some() {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    Ok(())
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn failed_job_retention_preserves_retried_and_recently_refailed_rows() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    for next_state in [jobs::JobState::Pending, jobs::JobState::Failed] {
        let job_id = queued_job(&database).await?;
        let claim = queue.claim(10, 30).await?.unwrap();
        queue.finish(&claim, Some(("permanent", false))).await?;
        job::Entity::update_many()
            .col_expr(
                job::Column::FinishedAt,
                Expr::value(chrono::Utc::now() - chrono::Duration::days(91)),
            )
            .filter(job::Column::Id.eq(job_id))
            .exec(&database.connection)
            .await?;
        let transaction = database.connection.begin().await?;
        job::Entity::find_by_id(job_id)
            .lock_exclusive()
            .one(&transaction)
            .await?;
        job::Entity::update_many()
            .col_expr(job::Column::State, Expr::value(next_state))
            .col_expr(
                job::Column::FinishedAt,
                Expr::value((next_state == jobs::JobState::Failed).then(chrono::Utc::now)),
            )
            .filter(job::Column::Id.eq(job_id))
            .exec(&transaction)
            .await?;
        let cleaner = queue.clone();
        let cleanup = tokio::spawn(async move { cleaner.cleanup_finished(30, 90).await });
        wait_for_delete_lock(&database).await?;
        transaction.commit().await?;
        assert_eq!(cleanup.await??, 0);
        assert!(
            job::Entity::find_by_id(job_id)
                .one(&database.connection)
                .await?
                .is_some()
        );
        if next_state == jobs::JobState::Pending {
            let claim = queue.claim(10, 30).await?.unwrap();
            queue.finish(&claim, None).await?;
        }
    }
    let removable = queued_job(&database).await?;
    let claim = queue.claim(10, 30).await?.unwrap();
    queue.finish(&claim, Some(("permanent", false))).await?;
    job::Entity::update_many()
        .col_expr(
            job::Column::FinishedAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::days(91)),
        )
        .filter(job::Column::Id.eq(removable))
        .exec(&database.connection)
        .await?;
    assert_eq!(queue.cleanup_finished(30, 90).await?, 1);
    assert!(!queue.retry_failed(removable).await?);
    assert!(
        job_attempt::Entity::find()
            .filter(job_attempt::Column::JobId.eq(removable))
            .all(&database.connection)
            .await?
            .is_empty()
    );
    database.finish().await
}
