use super::columns::{created_at, uuid_primary_key};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Accounts {
    Table,
    Id,
    Email,
    PasswordHash,
    Verified,
    Active,
    Administrator,
    SecurityVersion,
    TotpSecret,
    TotpLastStep,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .col(uuid_primary_key(Accounts::Id))
                    .col(
                        ColumnDef::new(Accounts::Email)
                            .string_len(254)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Accounts::PasswordHash).text())
                    .col(
                        ColumnDef::new(Accounts::Verified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Accounts::Active)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Accounts::Administrator)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Accounts::SecurityVersion)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Accounts::TotpSecret).text())
                    .col(
                        ColumnDef::new(Accounts::TotpLastStep)
                            .big_integer()
                            .not_null()
                            .default(-1),
                    )
                    .col(created_at(Accounts::CreatedAt))
                    .check(Expr::col(Accounts::Email).eq(Func::lower(Expr::col(Accounts::Email))))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Accounts::Table).to_owned())
            .await
    }
}
