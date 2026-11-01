use super::m20261013_000013_create_audit_events::AuditEvents;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("audit_events_retention")
                    .table(AuditEvents::Table)
                    .col(AuditEvents::CreatedAt)
                    .col(AuditEvents::Id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("audit_events_retention")
                    .table(AuditEvents::Table)
                    .to_owned(),
            )
            .await
    }
}
