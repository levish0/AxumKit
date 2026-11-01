use super::columns::{created_at, uuid_primary_key};
use super::m20261013_000001_create_accounts::Accounts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum AuditEvents {
    Table,
    Id,
    AccountId,
    Event,
    RequestId,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuditEvents::Table)
                    .col(uuid_primary_key(AuditEvents::Id))
                    .col(ColumnDef::new(AuditEvents::AccountId).uuid())
                    .col(ColumnDef::new(AuditEvents::Event).string_len(64).not_null())
                    .col(ColumnDef::new(AuditEvents::RequestId).uuid().not_null())
                    .col(created_at(AuditEvents::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(AuditEvents::Table, AuditEvents::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("audit_events_account")
                    .table(AuditEvents::Table)
                    .col(AuditEvents::AccountId)
                    .col(AuditEvents::Id)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuditEvents::Table).to_owned())
            .await
    }
}
