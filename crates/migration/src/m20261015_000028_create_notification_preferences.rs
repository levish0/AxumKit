use super::{columns::uuid_primary_key, m20261013_000001_create_accounts::Accounts};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum NotificationPreferences {
    Table,
    Id,
    AccountId,
    PostComments,
    PushEnabled,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NotificationPreferences::Table)
                    .col(uuid_primary_key(NotificationPreferences::Id))
                    .col(
                        ColumnDef::new(NotificationPreferences::AccountId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotificationPreferences::PostComments)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(NotificationPreferences::PushEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                NotificationPreferences::Table,
                                NotificationPreferences::AccountId,
                            )
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("notification_preferences_account")
                    .table(NotificationPreferences::Table)
                    .unique()
                    .col(NotificationPreferences::AccountId)
                    .to_owned(),
            )
            .await?;
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
