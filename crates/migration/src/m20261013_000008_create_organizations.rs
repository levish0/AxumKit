use super::columns::{created_at, uuid_primary_key};
use super::m20261013_000001_create_accounts::Accounts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Organizations {
    Table,
    Id,
    Name,
    OwnerId,
    Version,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Organizations::Table)
                    .col(uuid_primary_key(Organizations::Id))
                    .col(
                        ColumnDef::new(Organizations::Name)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Organizations::OwnerId).uuid().not_null())
                    .col(
                        ColumnDef::new(Organizations::Version)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(created_at(Organizations::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Organizations::Table, Organizations::OwnerId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Organizations::Table).to_owned())
            .await
    }
}
