use super::columns::uuid_primary_key;
use super::m20261013_000001_create_accounts::Accounts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum OauthFlows {
    Table,
    Id,
    StateHash,
    BindingHash,
    Provider,
    Secret,
    AccountId,
    SessionId,
    SecurityVersion,
    ExpiresAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OauthFlows::Table)
                    .col(uuid_primary_key(OauthFlows::Id))
                    .col(
                        ColumnDef::new(OauthFlows::StateHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(OauthFlows::BindingHash)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OauthFlows::Provider)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(ColumnDef::new(OauthFlows::Secret).text().not_null())
                    .col(ColumnDef::new(OauthFlows::AccountId).uuid())
                    .col(ColumnDef::new(OauthFlows::SessionId).uuid())
                    .col(ColumnDef::new(OauthFlows::SecurityVersion).big_integer())
                    .col(
                        ColumnDef::new(OauthFlows::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(OauthFlows::Table, OauthFlows::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("oauth_flows_expiry")
                    .table(OauthFlows::Table)
                    .col(OauthFlows::ExpiresAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OauthFlows::Table).to_owned())
            .await
    }
}
