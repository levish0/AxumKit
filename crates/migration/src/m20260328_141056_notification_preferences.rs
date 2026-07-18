use crate::m20250825_033639_users::Users;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NotificationPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NotificationPreferences::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(
                        ColumnDef::new(NotificationPreferences::UserId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(NotificationPreferences::EmailEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(NotificationPreferences::PushEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        timestamp_with_time_zone(NotificationPreferences::UpdatedAt)
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_preferences_user_id")
                            .from(
                                NotificationPreferences::Table,
                                NotificationPreferences::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // user_id already has a UNIQUE constraint, so an extra unique index is redundant.
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(NotificationPreferences::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub enum NotificationPreferences {
    Table,
    Id,
    UserId,
    EmailEnabled,
    PushEnabled,
    UpdatedAt,
}
