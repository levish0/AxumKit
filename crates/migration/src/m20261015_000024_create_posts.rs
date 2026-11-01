use super::{
    columns::{created_at, uuid_primary_key},
    m20261013_000001_create_accounts::Accounts,
    m20261013_000008_create_organizations::Organizations,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub(super) enum Posts {
    Table,
    Id,
    OrganizationId,
    AuthorId,
    Title,
    Body,
    State,
    Version,
    CreatedAt,
    UpdatedAt,
    SearchReconciledAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Posts::Table)
                    .col(uuid_primary_key(Posts::Id))
                    .col(ColumnDef::new(Posts::OrganizationId).uuid().not_null())
                    .col(ColumnDef::new(Posts::AuthorId).uuid().not_null())
                    .col(ColumnDef::new(Posts::Title).string_len(200).not_null())
                    .col(ColumnDef::new(Posts::Body).text().not_null())
                    .col(ColumnDef::new(Posts::State).string_len(16).not_null())
                    .col(
                        ColumnDef::new(Posts::Version)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(created_at(Posts::CreatedAt))
                    .col(created_at(Posts::UpdatedAt))
                    .col(ColumnDef::new(Posts::SearchReconciledAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Posts::Table, Posts::OrganizationId)
                            .to(Organizations::Table, Organizations::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Posts::Table, Posts::AuthorId)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .check(Expr::col(Posts::State).is_in(["draft", "published", "deleted"]))
                    .check(Expr::col(Posts::Version).gte(1))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("posts_organization_state_created")
                    .table(Posts::Table)
                    .col(Posts::OrganizationId)
                    .col(Posts::State)
                    .col(Posts::CreatedAt)
                    .col(Posts::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("posts_reconciliation")
                    .table(Posts::Table)
                    .col(Posts::SearchReconciledAt)
                    .col(Posts::Id)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Posts::Table).to_owned())
            .await
    }
}
