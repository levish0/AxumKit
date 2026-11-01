use super::m20261013_000013_create_audit_events::AuditEvents;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;
#[derive(DeriveIden)]
enum Resource {
    ResourceId,
    ResourceType,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AuditEvents::Table)
                    .add_column(ColumnDef::new(Resource::ResourceId).uuid())
                    .add_column(ColumnDef::new(Resource::ResourceType).string_len(32))
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AuditEvents::Table)
                    .drop_column(Resource::ResourceType)
                    .drop_column(Resource::ResourceId)
                    .to_owned(),
            )
            .await
    }
}
