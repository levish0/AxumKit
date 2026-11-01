use super::columns::{created_at, uuid_primary_key};
use super::m20261013_000011_create_jobs::Jobs;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum JobAttempts {
    Table,
    Id,
    JobId,
    Attempt,
    StartedAt,
    FinishedAt,
    Outcome,
    ErrorCode,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JobAttempts::Table)
                    .col(uuid_primary_key(JobAttempts::Id))
                    .col(ColumnDef::new(JobAttempts::JobId).uuid().not_null())
                    .col(ColumnDef::new(JobAttempts::Attempt).integer().not_null())
                    .col(created_at(JobAttempts::StartedAt))
                    .col(ColumnDef::new(JobAttempts::FinishedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(JobAttempts::Outcome).string_len(16))
                    .col(ColumnDef::new(JobAttempts::ErrorCode).string_len(100))
                    .check(Expr::col(JobAttempts::Outcome).is_in([
                        "succeeded",
                        "retry",
                        "failed",
                        "lease_expired",
                    ]))
                    .foreign_key(
                        ForeignKey::create()
                            .from(JobAttempts::Table, JobAttempts::JobId)
                            .to(Jobs::Table, Jobs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("job_attempt_number")
                    .table(JobAttempts::Table)
                    .col(JobAttempts::JobId)
                    .col(JobAttempts::Attempt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JobAttempts::Table).to_owned())
            .await
    }
}
