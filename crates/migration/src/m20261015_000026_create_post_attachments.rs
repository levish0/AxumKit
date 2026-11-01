use super::{columns::uuid_primary_key, m20261015_000024_create_posts::Posts};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum PostAttachments {
    Table,
    Id,
    PostId,
    FileId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PostAttachments::Table)
                    .col(uuid_primary_key(PostAttachments::Id))
                    .col(ColumnDef::new(PostAttachments::PostId).uuid().not_null())
                    .col(ColumnDef::new(PostAttachments::FileId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(PostAttachments::Table, PostAttachments::PostId)
                            .to(Posts::Table, Posts::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(PostAttachments::Table, PostAttachments::FileId)
                            .to(Alias::new("files"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("post_attachments_unique")
                    .table(PostAttachments::Table)
                    .unique()
                    .col(PostAttachments::PostId)
                    .col(PostAttachments::FileId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PostAttachments::Table).to_owned())
            .await
    }
}
