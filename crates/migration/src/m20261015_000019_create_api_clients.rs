use super::{
    columns::{created_at, uuid_primary_key},
    m20261013_000008_create_organizations::Organizations,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum ApiClients {
    Table,
    Id,
    OrganizationId,
    Name,
    Scopes,
    Active,
    MinuteLimit,
    MonthLimit,
    MinuteStartedAt,
    MonthStartedAt,
    MinuteUsed,
    MonthUsed,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiClients::Table)
                    .col(uuid_primary_key(ApiClients::Id))
                    .col(ColumnDef::new(ApiClients::OrganizationId).uuid().not_null())
                    .col(ColumnDef::new(ApiClients::Name).string_len(100).not_null())
                    .col(ColumnDef::new(ApiClients::Scopes).json_binary().not_null())
                    .col(
                        ColumnDef::new(ApiClients::Active)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ApiClients::MinuteLimit)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ApiClients::MonthLimit)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(ApiClients::MinuteStartedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(ApiClients::MonthStartedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(ApiClients::MinuteUsed)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ApiClients::MonthUsed)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(created_at(ApiClients::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(ApiClients::Table, ApiClients::OrganizationId)
                            .to(Organizations::Table, Organizations::Id),
                    )
                    .check(Expr::col(ApiClients::MinuteLimit).gte(0))
                    .check(Expr::col(ApiClients::MonthLimit).gte(0))
                    .check(Expr::col(ApiClients::MinuteUsed).gte(0))
                    .check(Expr::col(ApiClients::MonthUsed).gte(0))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("api_clients_organization_id")
                    .table(ApiClients::Table)
                    .col(ApiClients::OrganizationId)
                    .col(ApiClients::Id)
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiClients::Table).to_owned())
            .await
    }
}
