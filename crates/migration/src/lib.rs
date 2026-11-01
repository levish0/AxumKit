use sea_orm_migration::MigrationStatus;
use sea_orm_migration::prelude::*;
mod columns;
mod m20261013_000001_create_accounts;
mod m20261013_000002_create_sessions;
mod m20261013_000003_create_challenges;
mod m20261013_000004_create_recovery_codes;
mod m20261013_000005_create_external_identities;
mod m20261013_000006_create_oauth_flows;
mod m20261013_000007_create_rate_limits;
mod m20261013_000008_create_organizations;
mod m20261013_000009_create_memberships;
mod m20261013_000010_create_mail_deliveries;
mod m20261013_000011_create_jobs;
mod m20261013_000012_create_job_attempts;
mod m20261013_000013_create_audit_events;
mod m20261013_000014_create_files;
mod m20261015_000015_index_audit_retention;
mod m20261015_000016_index_job_retention;
mod m20261015_000017_route_scheduled_jobs;
mod m20261015_000018_allow_notification_deliveries;
mod m20261015_000019_create_api_clients;
mod m20261015_000020_create_api_keys;
mod m20261015_000021_create_api_usage;
mod m20261015_000022_identify_audit_resources;
mod m20261015_000023_create_profiles;
mod m20261015_000024_create_posts;
mod m20261015_000025_create_comments;
mod m20261015_000026_create_post_attachments;
mod m20261015_000027_create_notifications;
mod m20261015_000028_create_notification_preferences;
mod m20261015_000029_create_push_devices;
mod m20261015_000030_create_push_deliveries;
mod m20261018_000031_create_oauth_native_flows;

pub struct Migrator;

pub async fn pending_migration_count(
    database: &sea_orm::DatabaseConnection,
) -> Result<usize, DbErr> {
    use sea_orm::{AccessMode, IsolationLevel, TransactionTrait};
    let transaction = database
        .begin_with_config(
            Some(IsolationLevel::RepeatableRead),
            Some(AccessMode::ReadOnly),
        )
        .await?;
    let migrations = Migrator::get_migration_with_status_read_only(&transaction).await?;
    let mut pending = 0;
    for migration in migrations {
        match migration.status() {
            MigrationStatus::Pending => pending += 1,
            MigrationStatus::Applied if pending > 0 => {
                return Err(DbErr::Custom(
                    "migration history has an unapplied predecessor".into(),
                ));
            }
            MigrationStatus::Applied => {}
        }
    }
    transaction.commit().await?;
    Ok(pending)
}

pub async fn require_current_schema(database: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    if !Migrator::get_pending_migrations_read_only(database)
        .await?
        .is_empty()
    {
        return Err(DbErr::Custom(
            "database schema is not current; run migrations before starting the service".into(),
        ));
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261013_000001_create_accounts::Migration),
            Box::new(m20261013_000002_create_sessions::Migration),
            Box::new(m20261013_000003_create_challenges::Migration),
            Box::new(m20261013_000004_create_recovery_codes::Migration),
            Box::new(m20261013_000005_create_external_identities::Migration),
            Box::new(m20261013_000006_create_oauth_flows::Migration),
            Box::new(m20261013_000007_create_rate_limits::Migration),
            Box::new(m20261013_000008_create_organizations::Migration),
            Box::new(m20261013_000009_create_memberships::Migration),
            Box::new(m20261013_000010_create_mail_deliveries::Migration),
            Box::new(m20261013_000011_create_jobs::Migration),
            Box::new(m20261013_000012_create_job_attempts::Migration),
            Box::new(m20261013_000013_create_audit_events::Migration),
            Box::new(m20261013_000014_create_files::Migration),
            Box::new(m20261015_000015_index_audit_retention::Migration),
            Box::new(m20261015_000016_index_job_retention::Migration),
            Box::new(m20261015_000017_route_scheduled_jobs::Migration),
            Box::new(m20261015_000018_allow_notification_deliveries::Migration),
            Box::new(m20261015_000019_create_api_clients::Migration),
            Box::new(m20261015_000020_create_api_keys::Migration),
            Box::new(m20261015_000021_create_api_usage::Migration),
            Box::new(m20261015_000022_identify_audit_resources::Migration),
            Box::new(m20261015_000023_create_profiles::Migration),
            Box::new(m20261015_000024_create_posts::Migration),
            Box::new(m20261015_000025_create_comments::Migration),
            Box::new(m20261015_000026_create_post_attachments::Migration),
            Box::new(m20261015_000027_create_notifications::Migration),
            Box::new(m20261015_000028_create_notification_preferences::Migration),
            Box::new(m20261015_000029_create_push_devices::Migration),
            Box::new(m20261015_000030_create_push_deliveries::Migration),
            Box::new(m20261018_000031_create_oauth_native_flows::Migration),
        ]
    }
}

pub async fn migrate(database: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::{
        ConnectionTrait, TransactionTrait,
        sea_query::{Func, Query},
    };
    let transaction = database.begin().await?;
    let migration_lock = Query::select()
        .expr(Func::cust("pg_advisory_xact_lock").arg(7_423_186_901_i64))
        .to_owned();
    transaction.query_one(&migration_lock).await?;
    Migrator::up(&transaction, None).await?;
    transaction.commit().await
}
