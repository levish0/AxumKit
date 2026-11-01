use super::{
    columns::{created_at, uuid_primary_key},
    m20261015_000019_create_api_clients::ApiClients,
    m20261015_000020_create_api_keys::ApiKeys,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;
#[derive(DeriveIden)]
pub(super) enum ApiUsage {
    Table,
    Id,
    ClientId,
    KeyId,
    Operation,
    RequestId,
    AdmittedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiUsage::Table)
                    .col(uuid_primary_key(ApiUsage::Id))
                    .col(ColumnDef::new(ApiUsage::ClientId).uuid().not_null())
                    .col(ColumnDef::new(ApiUsage::KeyId).uuid().not_null())
                    .col(
                        ColumnDef::new(ApiUsage::Operation)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ApiUsage::RequestId).uuid().not_null())
                    .col(created_at(ApiUsage::AdmittedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(ApiUsage::Table, ApiUsage::ClientId)
                            .to(ApiClients::Table, ApiClients::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ApiUsage::Table, ApiUsage::KeyId)
                            .to(ApiKeys::Table, ApiKeys::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("api_usage_client_time")
                    .table(ApiUsage::Table)
                    .col(ApiUsage::ClientId)
                    .col(ApiUsage::AdmittedAt)
                    .col(ApiUsage::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("api_usage_retention")
                    .table(ApiUsage::Table)
                    .col(ApiUsage::AdmittedAt)
                    .col(ApiUsage::Id)
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiUsage::Table).to_owned())
            .await
    }
}
