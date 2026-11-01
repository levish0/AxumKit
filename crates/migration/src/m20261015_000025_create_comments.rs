use super::{
    columns::{created_at, uuid_primary_key},
    m20261013_000001_create_accounts::Accounts,
    m20261015_000024_create_posts::Posts,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Comments {
    Table,
    Id,
    PostId,
    AuthorId,
    Body,
    Deleted,
    Version,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Comments::Table)
                    .col(uuid_primary_key(Comments::Id))
                    .col(ColumnDef::new(Comments::PostId).uuid().not_null())
                    .col(ColumnDef::new(Comments::AuthorId).uuid().not_null())
                    .col(ColumnDef::new(Comments::Body).text().not_null())
                    .col(
                        ColumnDef::new(Comments::Deleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Comments::Version)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(created_at(Comments::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Comments::Table, Comments::PostId)
                            .to(Posts::Table, Posts::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Comments::Table, Comments::AuthorId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .check(Expr::col(Comments::Version).gte(1))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("comments_post_created")
                    .table(Comments::Table)
                    .col(Comments::PostId)
                    .col(Comments::CreatedAt)
                    .col(Comments::Id)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Comments::Table).to_owned())
            .await
    }
}
