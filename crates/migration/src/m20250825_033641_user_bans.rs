use crate::m20250825_033639_users::Users;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserBans::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserBans::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(
                        ColumnDef::new(UserBans::UserId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(UserBans::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserBans::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_bans_user")
                            .from(UserBans::Table, UserBans::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_bans_expires_at")
                    .table(UserBans::Table)
                    .col(UserBans::ExpiresAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserBans::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum UserBans {
    Table,
    Id,
    UserId,
    ExpiresAt,
    CreatedAt,
}
