use super::{
    columns::{created_at, uuid_primary_key},
    m20261013_000001_create_accounts::Accounts,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Profiles {
    Table,
    Id,
    AccountId,
    DisplayName,
    Bio,
    Version,
    CreatedAt,
    UpdatedAt,
    SearchReconciledAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Profiles::Table)
                    .col(uuid_primary_key(Profiles::Id))
                    .col(ColumnDef::new(Profiles::AccountId).uuid().not_null())
                    .col(
                        ColumnDef::new(Profiles::DisplayName)
                            .string_len(80)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Profiles::Bio).text().not_null())
                    .col(
                        ColumnDef::new(Profiles::Version)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(created_at(Profiles::CreatedAt))
                    .col(created_at(Profiles::UpdatedAt))
                    .col(ColumnDef::new(Profiles::SearchReconciledAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Profiles::Table, Profiles::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .check(Expr::col(Profiles::Version).gte(1))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("profiles_account")
                    .table(Profiles::Table)
                    .unique()
                    .col(Profiles::AccountId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("profiles_reconciliation")
                    .table(Profiles::Table)
                    .col(Profiles::SearchReconciledAt)
                    .col(Profiles::Id)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Profiles::Table).to_owned())
            .await
    }
}
