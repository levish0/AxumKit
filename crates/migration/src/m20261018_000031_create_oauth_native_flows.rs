use super::columns::uuid_primary_key;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum OauthNativeFlows {
    Table,
    Id,
    FlowHash,
    NonceHash,
    Provider,
    ExpiresAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OauthNativeFlows::Table)
                    .col(uuid_primary_key(OauthNativeFlows::Id))
                    .col(
                        ColumnDef::new(OauthNativeFlows::FlowHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(OauthNativeFlows::NonceHash)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OauthNativeFlows::Provider)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OauthNativeFlows::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("oauth_native_flows_expiry")
                    .table(OauthNativeFlows::Table)
                    .col(OauthNativeFlows::ExpiresAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OauthNativeFlows::Table).to_owned())
            .await
    }
}
