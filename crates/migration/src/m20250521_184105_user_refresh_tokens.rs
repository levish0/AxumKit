use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserRefreshTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserRefreshTokens::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserRefreshTokens::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserRefreshTokens::RefreshToken)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRefreshTokens::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRefreshTokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRefreshTokens::RevokedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserRefreshTokens::Table, UserRefreshTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserRefreshTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserRefreshTokens {
    Table,
    Id,
    UserId,
    RefreshToken,
    ExpiresAt,
    CreatedAt,
    RevokedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
