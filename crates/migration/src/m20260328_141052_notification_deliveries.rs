use crate::m20250825_033639_users::Users;
use crate::m20260328_141050_notification_events::NotificationEvents;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NotificationDeliveries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NotificationDeliveries::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::EventId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        timestamp_with_time_zone(NotificationDeliveries::CreatedAt)
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::ReadAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_deliveries_user_id")
                            .from(
                                NotificationDeliveries::Table,
                                NotificationDeliveries::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_deliveries_event_id")
                            .from(
                                NotificationDeliveries::Table,
                                NotificationDeliveries::EventId,
                            )
                            .to(NotificationEvents::Table, NotificationEvents::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_deliveries_user_unread")
                    .table(NotificationDeliveries::Table)
                    .col(NotificationDeliveries::UserId)
                    .col(NotificationDeliveries::IsRead)
                    .col(NotificationDeliveries::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_deliveries_user_id")
                    .table(NotificationDeliveries::Table)
                    .col(NotificationDeliveries::UserId)
                    .col(NotificationDeliveries::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_deliveries_created_at")
                    .table(NotificationDeliveries::Table)
                    .col(NotificationDeliveries::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_deliveries_event_id")
                    .table(NotificationDeliveries::Table)
                    .col(NotificationDeliveries::EventId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(NotificationDeliveries::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub enum NotificationDeliveries {
    Table,
    Id,
    UserId,
    EventId,
    IsRead,
    CreatedAt,
    ReadAt,
}
