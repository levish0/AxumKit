use super::columns::uuid_primary_key;
use super::m20261013_000001_create_accounts::Accounts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum ExternalIdentities {
    Table,
    Id,
    Issuer,
    Subject,
    AccountId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExternalIdentities::Table)
                    .col(uuid_primary_key(ExternalIdentities::Id))
                    .col(ColumnDef::new(ExternalIdentities::Issuer).text().not_null())
                    .col(
                        ColumnDef::new(ExternalIdentities::Subject)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExternalIdentities::AccountId)
                            .uuid()
                            .not_null(),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("external_identity_unique")
                            .col(ExternalIdentities::Issuer)
                            .col(ExternalIdentities::Subject),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ExternalIdentities::Table, ExternalIdentities::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("external_identities_account")
                    .table(ExternalIdentities::Table)
                    .col(ExternalIdentities::AccountId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExternalIdentities::Table).to_owned())
            .await
    }
}
