use super::columns::{created_at, uuid_primary_key};
use super::m20261013_000001_create_accounts::Accounts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum MailDeliveries {
    Table,
    Id,
    AccountId,
    ChallengeHash,
    Ciphertext,
    State,
    ExpiresAt,
    AcceptedAt,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MailDeliveries::Table)
                    .col(uuid_primary_key(MailDeliveries::Id))
                    .col(ColumnDef::new(MailDeliveries::AccountId).uuid().not_null())
                    .col(
                        ColumnDef::new(MailDeliveries::ChallengeHash)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(ColumnDef::new(MailDeliveries::Ciphertext).text())
                    .col(
                        ColumnDef::new(MailDeliveries::State)
                            .string_len(16)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(MailDeliveries::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(MailDeliveries::AcceptedAt).timestamp_with_time_zone())
                    .col(created_at(MailDeliveries::CreatedAt))
                    .check(
                        Expr::col(MailDeliveries::State).is_in(["pending", "accepted", "expired"]),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(MailDeliveries::Table, MailDeliveries::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("mail_deliveries_expiry")
                    .table(MailDeliveries::Table)
                    .col(MailDeliveries::ExpiresAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MailDeliveries::Table).to_owned())
            .await
    }
}
