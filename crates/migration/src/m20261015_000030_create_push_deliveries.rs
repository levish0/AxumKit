use super::{
    columns::{created_at, uuid_primary_key},
    m20261015_000027_create_notifications::Notifications,
    m20261015_000029_create_push_devices::PushDevices,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum PushDeliveries {
    Table,
    Id,
    NotificationId,
    DeviceId,
    DeviceVersion,
    State,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PushDeliveries::Table)
                    .col(uuid_primary_key(PushDeliveries::Id))
                    .col(
                        ColumnDef::new(PushDeliveries::NotificationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PushDeliveries::DeviceId).uuid().not_null())
                    .col(
                        ColumnDef::new(PushDeliveries::DeviceVersion)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushDeliveries::State)
                            .string_len(16)
                            .not_null()
                            .default("pending"),
                    )
                    .col(created_at(PushDeliveries::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(PushDeliveries::Table, PushDeliveries::NotificationId)
                            .to(Notifications::Table, Notifications::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(PushDeliveries::Table, PushDeliveries::DeviceId)
                            .to(PushDevices::Table, PushDevices::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .check(Expr::col(PushDeliveries::State).is_in(["pending", "sent", "skipped"]))
                    .check(Expr::col(PushDeliveries::DeviceVersion).gte(1))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("push_deliveries_notification_device")
                    .table(PushDeliveries::Table)
                    .unique()
                    .col(PushDeliveries::NotificationId)
                    .col(PushDeliveries::DeviceId)
                    .col(PushDeliveries::DeviceVersion)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PushDeliveries::Table).to_owned())
            .await
    }
}
