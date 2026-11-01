use super::columns::{created_at, uuid_primary_key};
use super::m20261013_000001_create_accounts::Accounts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Challenges {
    Table,
    Id,
    TokenHash,
    AccountId,
    Purpose,
    SecurityVersion,
    Secret,
    ExpiresAt,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Challenges::Table)
                    .col(uuid_primary_key(Challenges::Id))
                    .col(
                        ColumnDef::new(Challenges::TokenHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Challenges::AccountId).uuid().not_null())
                    .col(
                        ColumnDef::new(Challenges::Purpose)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Challenges::SecurityVersion)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Challenges::Secret).text())
                    .col(
                        ColumnDef::new(Challenges::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(created_at(Challenges::CreatedAt))
                    .check(Expr::col(Challenges::Purpose).is_in([
                        "verify_email",
                        "reset_password",
                        "mfa",
                        "totp_setup",
                    ]))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Challenges::Table, Challenges::AccountId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("challenges_account")
                    .table(Challenges::Table)
                    .col(Challenges::AccountId)
                    .col(Challenges::Purpose)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("challenges_expiry")
                    .table(Challenges::Table)
                    .col(Challenges::ExpiresAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Challenges::Table).to_owned())
            .await
    }
}
