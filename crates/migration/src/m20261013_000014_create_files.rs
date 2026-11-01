use super::{
    columns::{created_at, uuid_primary_key},
    m20261013_000001_create_accounts::Accounts,
    m20261013_000008_create_organizations::Organizations,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Files {
    Table,
    Id,
    OrganizationId,
    UploaderId,
    ObjectKey,
    Filename,
    MediaType,
    ByteSize,
    State,
    Version,
    ContentHash,
    UploadExpiresAt,
    CreatedAt,
    UpdatedAt,
    SearchReconciledAt,
    SearchDocumentMayExist,
    CleanupScheduledAt,
    CleanupCompletedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Files::Table)
                    .col(uuid_primary_key(Files::Id))
                    .col(ColumnDef::new(Files::OrganizationId).uuid().not_null())
                    .col(ColumnDef::new(Files::UploaderId).uuid().not_null())
                    .col(
                        ColumnDef::new(Files::ObjectKey)
                            .string_len(150)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Files::Filename).string_len(255).not_null())
                    .col(ColumnDef::new(Files::MediaType).string_len(120))
                    .col(ColumnDef::new(Files::ByteSize).big_integer().not_null())
                    .col(ColumnDef::new(Files::State).string_len(20).not_null())
                    .col(
                        ColumnDef::new(Files::Version)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(Files::ContentHash).string_len(64))
                    .col(
                        ColumnDef::new(Files::UploadExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(created_at(Files::CreatedAt))
                    .col(created_at(Files::UpdatedAt))
                    .col(ColumnDef::new(Files::SearchReconciledAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Files::SearchDocumentMayExist)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Files::CleanupScheduledAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Files::CleanupCompletedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Files::Table, Files::OrganizationId)
                            .to(Organizations::Table, Organizations::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Files::Table, Files::UploaderId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .check(Expr::col(Files::ByteSize).gte(0))
                    .check(Expr::col(Files::Version).gt(0))
                    .check(Expr::col(Files::State).is_in([
                        "uploading",
                        "transferring",
                        "ready",
                        "deleting",
                        "deleted",
                    ]))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("files_organization_id_state_id")
                    .table(Files::Table)
                    .col(Files::OrganizationId)
                    .col(Files::State)
                    .col(Files::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("files_state_upload_expires_at")
                    .table(Files::Table)
                    .col(Files::State)
                    .col(Files::UploadExpiresAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("files_search_reconciled_at_id")
                    .table(Files::Table)
                    .col(Files::SearchReconciledAt)
                    .col(Files::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("files_uploader_id_created_at")
                    .table(Files::Table)
                    .col(Files::UploaderId)
                    .col(Files::CreatedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("files_cleanup_scheduled_at_id")
                    .table(Files::Table)
                    .col(Files::CleanupScheduledAt)
                    .col(Files::Id)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Files::Table).to_owned())
            .await
    }
}
