use super::m20261013_000011_create_jobs::Jobs;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Routing {
    Queue,
    ScheduleKey,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .add_column(
                        ColumnDef::new(Routing::Queue)
                            .string_len(64)
                            .not_null()
                            .default("application"),
                    )
                    .add_column(ColumnDef::new(Routing::ScheduleKey).string_len(128))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("jobs_schedule_bucket")
                    .table(Jobs::Table)
                    .col(Routing::Queue)
                    .col(Routing::ScheduleKey)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("jobs_queue_ready")
                    .table(Jobs::Table)
                    .col(Routing::Queue)
                    .col(Jobs::AvailableAt)
                    .col(Jobs::Id)
                    .and_where(Expr::col(Jobs::State).eq("pending"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("jobs_queue_expired")
                    .table(Jobs::Table)
                    .col(Routing::Queue)
                    .col(Jobs::LeaseUntil)
                    .col(Jobs::Id)
                    .and_where(Expr::col(Jobs::State).eq("running"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in [
            "jobs_queue_expired",
            "jobs_queue_ready",
            "jobs_schedule_bucket",
        ] {
            manager
                .drop_index(Index::drop().table(Jobs::Table).name(name).to_owned())
                .await?;
        }
        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .drop_column(Routing::ScheduleKey)
                    .drop_column(Routing::Queue)
                    .to_owned(),
            )
            .await
    }
}
