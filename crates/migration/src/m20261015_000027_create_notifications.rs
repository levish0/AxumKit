use super::{
    columns::{created_at, uuid_primary_key},
    m20261013_000001_create_accounts::Accounts,
    m20261013_000008_create_organizations::Organizations,
    m20261015_000024_create_posts::Posts,
    m20261015_000025_create_comments::Comments,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Notifications {
    Table,
    Id,
    RecipientId,
    ActorId,
    OrganizationId,
    PostId,
    CommentId,
    CreatedAt,
    ReadAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notifications::Table)
                    .col(uuid_primary_key(Notifications::Id))
                    .col(ColumnDef::new(Notifications::RecipientId).uuid().not_null())
                    .col(ColumnDef::new(Notifications::ActorId).uuid().not_null())
                    .col(
                        ColumnDef::new(Notifications::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Notifications::PostId).uuid().not_null())
                    .col(ColumnDef::new(Notifications::CommentId).uuid().not_null())
                    .col(created_at(Notifications::CreatedAt))
                    .col(ColumnDef::new(Notifications::ReadAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Notifications::Table, Notifications::RecipientId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Notifications::Table, Notifications::ActorId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Notifications::Table, Notifications::OrganizationId)
                            .to(Organizations::Table, Organizations::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Notifications::Table, Notifications::PostId)
                            .to(Posts::Table, Posts::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Notifications::Table, Notifications::CommentId)
                            .to(Comments::Table, Comments::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("notifications_comment_recipient")
                    .table(Notifications::Table)
                    .unique()
                    .col(Notifications::CommentId)
                    .col(Notifications::RecipientId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("notifications_recipient_created")
                    .table(Notifications::Table)
                    .col(Notifications::RecipientId)
                    .col(Notifications::CreatedAt)
                    .col(Notifications::Id)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Notifications::Table).to_owned())
            .await
    }
}
