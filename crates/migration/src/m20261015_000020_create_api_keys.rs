use super::{
    columns::{created_at, uuid_primary_key},
    m20261015_000019_create_api_clients::ApiClients,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;
#[derive(DeriveIden)]
pub(super) enum ApiKeys {
    Table,
    Id,
    ClientId,
    Name,
    SecretHash,
    CreatedAt,
    ExpiresAt,
    RevokedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .col(uuid_primary_key(ApiKeys::Id))
                    .col(ColumnDef::new(ApiKeys::ClientId).uuid().not_null())
                    .col(ColumnDef::new(ApiKeys::Name).string_len(100).not_null())
                    .col(
                        ColumnDef::new(ApiKeys::SecretHash)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(created_at(ApiKeys::CreatedAt))
                    .col(
                        ColumnDef::new(ApiKeys::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ApiKeys::RevokedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .from(ApiKeys::Table, ApiKeys::ClientId)
                            .to(ApiClients::Table, ApiClients::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("api_keys_client_id")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::ClientId)
                    .col(ApiKeys::Id)
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKeys::Table).to_owned())
            .await
    }
}
