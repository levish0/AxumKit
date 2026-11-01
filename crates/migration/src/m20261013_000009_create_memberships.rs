use super::columns::uuid_primary_key;
use super::m20261013_000001_create_accounts::Accounts;
use super::m20261013_000008_create_organizations::Organizations;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Memberships {
    Table,
    Id,
    OrganizationId,
    AccountId,
    Role,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Memberships::Table)
                    .col(uuid_primary_key(Memberships::Id))
                    .col(
                        ColumnDef::new(Memberships::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Memberships::AccountId).uuid().not_null())
                    .col(ColumnDef::new(Memberships::Role).string_len(16).not_null())
                    .index(
                        Index::create()
                            .unique()
                            .name("membership_unique")
                            .col(Memberships::OrganizationId)
                            .col(Memberships::AccountId),
                    )
                    .check(Expr::col(Memberships::Role).is_in(["owner", "member"]))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Memberships::Table, Memberships::OrganizationId)
                            .to(Organizations::Table, Organizations::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Memberships::Table, Memberships::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("memberships_account")
                    .table(Memberships::Table)
                    .col(Memberships::AccountId)
                    .col(Memberships::OrganizationId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Memberships::Table).to_owned())
            .await
    }
}
