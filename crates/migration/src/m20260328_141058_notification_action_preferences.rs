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
                    .table(NotificationActionPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NotificationActionPreferences::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(
                        ColumnDef::new(NotificationActionPreferences::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotificationActionPreferences::Action)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotificationActionPreferences::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_action_preferences_user_id")
                            .from(
                                NotificationActionPreferences::Table,
                                NotificationActionPreferences::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint to prevent duplicate preferences
        manager
            .create_index(
                Index::create()
                    .name("idx_notification_action_preferences_unique")
                    .table(NotificationActionPreferences::Table)
                    .col(NotificationActionPreferences::UserId)
                    .col(NotificationActionPreferences::Action)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // (user_id, action) unique index also covers user_id-only lookups (leftmost prefix).
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(NotificationActionPreferences::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub enum NotificationActionPreferences {
    Table,
    Id,
    UserId,
    Action,
    Enabled,
}
