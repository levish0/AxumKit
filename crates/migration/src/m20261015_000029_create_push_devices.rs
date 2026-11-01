use super::{
    columns::{created_at, uuid_primary_key},
    m20261013_000001_create_accounts::Accounts,
    m20261013_000002_create_sessions::Sessions,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum PushDevices {
    Table,
    Id,
    AccountId,
    SessionId,
    TokenHash,
    EncryptedToken,
    Platform,
    Version,
    ExpiresAt,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PushDevices::Table)
                    .col(uuid_primary_key(PushDevices::Id))
                    .col(ColumnDef::new(PushDevices::AccountId).uuid().not_null())
                    .col(ColumnDef::new(PushDevices::SessionId).uuid().not_null())
                    .col(
                        ColumnDef::new(PushDevices::TokenHash)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushDevices::EncryptedToken)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushDevices::Platform)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushDevices::Version)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(PushDevices::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(created_at(PushDevices::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(PushDevices::Table, PushDevices::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(PushDevices::Table, PushDevices::SessionId)
                            .to(Sessions::Table, Sessions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .check(Expr::col(PushDevices::Platform).is_in(["android", "ios", "web"]))
                    .check(Expr::col(PushDevices::Version).gte(1))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("push_devices_token")
                    .table(PushDevices::Table)
                    .unique()
                    .col(PushDevices::TokenHash)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("push_devices_account")
                    .table(PushDevices::Table)
                    .col(PushDevices::AccountId)
                    .col(PushDevices::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("push_devices_expiry")
                    .table(PushDevices::Table)
                    .col(PushDevices::ExpiresAt)
                    .col(PushDevices::Id)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PushDevices::Table).to_owned())
            .await
    }
}
