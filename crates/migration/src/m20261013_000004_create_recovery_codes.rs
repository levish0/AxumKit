use super::columns::uuid_primary_key;
use super::m20261013_000001_create_accounts::Accounts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum RecoveryCodes {
    Table,
    Id,
    AccountId,
    CodeHash,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RecoveryCodes::Table)
                    .col(uuid_primary_key(RecoveryCodes::Id))
                    .col(ColumnDef::new(RecoveryCodes::AccountId).uuid().not_null())
                    .col(
                        ColumnDef::new(RecoveryCodes::CodeHash)
                            .string_len(64)
                            .not_null(),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("recovery_codes_unique")
                            .col(RecoveryCodes::AccountId)
                            .col(RecoveryCodes::CodeHash),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RecoveryCodes::Table, RecoveryCodes::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecoveryCodes::Table).to_owned())
            .await
    }
}
