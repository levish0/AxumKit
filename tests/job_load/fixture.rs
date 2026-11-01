use anyhow::{Context, Result, ensure};
use jobs::{Claim, TraceContext, entities::job};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseBackend,
    DatabaseConnection, EntityTrait, ExprTrait, QueryFilter, Schema, Set, TransactionTrait,
    sea_query::Expr,
};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub const BULK_JOB: &str = "load.bulk_projection";
pub const NOTIFICATION_JOB: &str = "load.notification";

pub mod record {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "job_load_records")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub version: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub async fn connect(url: &str) -> Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(url);
    options.max_connections(8).sqlx_logging(false);
    Ok(Database::connect(options).await?)
}

pub async fn prepare(database: &DatabaseConnection, history_rows: usize) -> Result<Uuid> {
    database
        .execute(&Schema::new(DatabaseBackend::Postgres).create_table_from_entity(record::Entity))
        .await?;
    let probe_id = Uuid::now_v7();
    record::ActiveModel {
        id: Set(probe_id),
        version: Set(0),
    }
    .insert(database)
    .await?;
    let old = chrono::Utc::now() - chrono::Duration::days(1);
    for chunk in (0..history_rows).collect::<Vec<_>>().chunks(250) {
        job::Entity::insert_many(chunk.iter().map(|_| job::ActiveModel {
            id: Set(Uuid::now_v7()),
            kind: Set("load.retained_history".into()),
            queue: Set("application".into()),
            payload: Set(serde_json::json!({})),
            request_id: Set(Uuid::now_v7()),
            max_attempts: Set(5),
            state: Set(jobs::JobState::Succeeded),
            created_at: Set(old),
            finished_at: Set(Some(old)),
            ..Default::default()
        }))
        .exec(database)
        .await?;
    }
    Ok(probe_id)
}

pub async fn produce(
    database: &DatabaseConnection,
    count: usize,
    producer: usize,
    epoch: Instant,
) -> Result<Vec<f64>> {
    let mut registration_ms = Vec::with_capacity(count);
    for index in 0..count {
        let started = Instant::now();
        let registration_started_ns = u64::try_from(started.duration_since(epoch).as_nanos())?;
        let transaction = database.begin().await?;
        let job_id = Uuid::now_v7();
        record::ActiveModel {
            id: Set(job_id),
            version: Set(0),
        }
        .insert(&transaction)
        .await?;
        let kind = if (index + producer).is_multiple_of(16) {
            NOTIFICATION_JOB
        } else {
            BULK_JOB
        };
        jobs::enqueue(
            &transaction,
            job_id,
            kind,
            &serde_json::json!({"registration_started_ns": registration_started_ns}),
            TraceContext::default(),
        )
        .await?;
        transaction.commit().await?;
        registration_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    Ok(registration_ms)
}

pub async fn handle(database: &DatabaseConnection, claim: &Claim, epoch: Instant) -> Result<f64> {
    let entered_at = epoch.elapsed();
    let registered_at = Duration::from_nanos(
        claim.payload["registration_started_ns"]
            .as_u64()
            .context("registration clock missing")?,
    );
    let registration_to_handler_ms = entered_at
        .checked_sub(registered_at)
        .context("invalid monotonic registration clock")?
        .as_secs_f64()
        * 1000.0;
    if claim.kind == BULK_JOB {
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let updated = record::Entity::update_many()
        .col_expr(
            record::Column::Version,
            Expr::col(record::Column::Version).add(1_i64),
        )
        .filter(record::Column::Id.eq(claim.id))
        .exec(database)
        .await?;
    ensure!(updated.rows_affected == 1, "job source is missing");
    Ok(registration_to_handler_ms)
}

pub async fn foreground_transaction(database: &DatabaseConnection, probe_id: Uuid) -> Result<f64> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let started = Instant::now();
        let transaction = database.begin().await?;
        record::Entity::update_many()
            .col_expr(
                record::Column::Version,
                Expr::col(record::Column::Version).add(1_i64),
            )
            .filter(record::Column::Id.eq(probe_id))
            .exec(&transaction)
            .await?;
        let probe = record::Entity::find_by_id(probe_id)
            .one(&transaction)
            .await?
            .context("probe missing")?;
        ensure!(probe.version > 0);
        transaction.commit().await?;
        Ok(started.elapsed().as_secs_f64() * 1000.0)
    })
    .await
    .context("foreground transaction exceeded five seconds")?
}
