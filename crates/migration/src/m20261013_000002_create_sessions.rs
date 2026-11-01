use super::columns::{created_at, uuid_primary_key};
use super::m20261013_000001_create_accounts::Accounts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Sessions {
    Table,
    Id,
    AccountId,
    TokenHash,
    SecurityVersion,
    CreatedAt,
    AuthenticatedAt,
    LastSeenAt,
    IdleExpiresAt,
    ExpiresAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .col(uuid_primary_key(Sessions::Id))
                    .col(ColumnDef::new(Sessions::AccountId).uuid().not_null())
                    .col(
                        ColumnDef::new(Sessions::TokenHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Sessions::SecurityVersion)
                            .big_integer()
                            .not_null(),
                    )
                    .col(created_at(Sessions::CreatedAt))
                    .col(created_at(Sessions::AuthenticatedAt))
                    .col(created_at(Sessions::LastSeenAt))
                    .col(
                        ColumnDef::new(Sessions::IdleExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Sessions::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Sessions::Table, Sessions::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("sessions_account")
                    .table(Sessions::Table)
                    .col(Sessions::AccountId)
                    .col(Sessions::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("sessions_expiry")
                    .table(Sessions::Table)
                    .col(Sessions::ExpiresAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}
