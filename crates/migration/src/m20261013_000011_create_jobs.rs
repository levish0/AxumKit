use super::columns::{created_at, uuid_primary_key};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Jobs {
    Table,
    Id,
    Kind,
    Payload,
    RequestId,
    State,
    Attempts,
    MaxAttempts,
    AvailableAt,
    ClaimToken,
    LeaseUntil,
    DeadlineAt,
    LastError,
    CreatedAt,
    FinishedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Jobs::Table)
                    .col(uuid_primary_key(Jobs::Id))
                    .col(ColumnDef::new(Jobs::Kind).string_len(100).not_null())
                    .col(ColumnDef::new(Jobs::Payload).json_binary().not_null())
                    .col(ColumnDef::new(Jobs::RequestId).uuid().not_null())
                    .col(
                        ColumnDef::new(Jobs::State)
                            .string_len(16)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(Jobs::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Jobs::MaxAttempts).integer().not_null())
                    .col(created_at(Jobs::AvailableAt))
                    .col(ColumnDef::new(Jobs::ClaimToken).uuid())
                    .col(ColumnDef::new(Jobs::LeaseUntil).timestamp_with_time_zone())
                    .col(ColumnDef::new(Jobs::DeadlineAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Jobs::LastError).string_len(100))
                    .col(created_at(Jobs::CreatedAt))
                    .col(ColumnDef::new(Jobs::FinishedAt).timestamp_with_time_zone())
                    .check(Expr::col(Jobs::State).is_in([
                        "pending",
                        "running",
                        "succeeded",
                        "failed",
                    ]))
                    .check(Expr::col(Jobs::Attempts).gte(0))
                    .check(Expr::col(Jobs::Attempts).lte(Expr::col(Jobs::MaxAttempts)))
                    .check(Expr::col(Jobs::MaxAttempts).between(1, 1000))
                    .check(
                        Expr::col(Jobs::State)
                            .eq("running")
                            .and(Expr::col(Jobs::ClaimToken).is_not_null())
                            .and(Expr::col(Jobs::LeaseUntil).is_not_null())
                            .and(Expr::col(Jobs::DeadlineAt).is_not_null())
                            .or(Expr::col(Jobs::State)
                                .ne("running")
                                .and(Expr::col(Jobs::ClaimToken).is_null())
                                .and(Expr::col(Jobs::LeaseUntil).is_null())
                                .and(Expr::col(Jobs::DeadlineAt).is_null())),
                    )
                    .check(Expr::col(Jobs::LeaseUntil).lte(Expr::col(Jobs::DeadlineAt)))
                    .check(
                        Expr::col(Jobs::State)
                            .is_in(["succeeded", "failed"])
                            .eq(Expr::col(Jobs::FinishedAt).is_not_null()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("jobs_ready")
                    .table(Jobs::Table)
                    .col(Jobs::AvailableAt)
                    .col(Jobs::Id)
                    .and_where(Expr::col(Jobs::State).eq("pending"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("jobs_expired")
                    .table(Jobs::Table)
                    .col(Jobs::LeaseUntil)
                    .col(Jobs::Id)
                    .and_where(Expr::col(Jobs::State).eq("running"))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Jobs::Table).to_owned())
            .await
    }
}
