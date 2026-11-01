use super::m20261013_000011_create_jobs::Jobs;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("jobs_retention")
                    .table(Jobs::Table)
                    .col(Jobs::State)
                    .col(Jobs::FinishedAt)
                    .col(Jobs::Id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("jobs_retention")
                    .table(Jobs::Table)
                    .to_owned(),
            )
            .await
    }
}
