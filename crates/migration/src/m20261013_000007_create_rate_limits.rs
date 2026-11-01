use super::columns::uuid_primary_key;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum RateLimits {
    Table,
    Id,
    Key,
    Attempts,
    ExpiresAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RateLimits::Table)
                    .col(uuid_primary_key(RateLimits::Id))
                    .col(
                        ColumnDef::new(RateLimits::Key)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(RateLimits::Attempts).integer().not_null())
                    .col(
                        ColumnDef::new(RateLimits::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .check(Expr::col(RateLimits::Attempts).gt(0))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("rate_limits_expiry")
                    .table(RateLimits::Table)
                    .col(RateLimits::ExpiresAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RateLimits::Table).to_owned())
            .await
    }
}
