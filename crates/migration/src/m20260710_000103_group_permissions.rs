use crate::m20250825_033639_users::Users;
use crate::m20260710_000101_groups::Groups;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GroupPermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GroupPermissions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(GroupPermissions::GroupId).uuid().not_null())
                    // Permission codename stored as TEXT (not a PG enum): the
                    // permission set grows with features, so additions must not
                    // require an enum migration.
                    .col(
                        ColumnDef::new(GroupPermissions::Permission)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GroupPermissions::CreatedBy).uuid().null())
                    .col(
                        ColumnDef::new(GroupPermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_permissions_group")
                            .from(GroupPermissions::Table, GroupPermissions::GroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_permissions_created_by")
                            .from(GroupPermissions::Table, GroupPermissions::CreatedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // One grant per (group, permission).
        manager
            .create_index(
                Index::create()
                    .name("uq_group_permissions_group_permission")
                    .table(GroupPermissions::Table)
                    .col(GroupPermissions::GroupId)
                    .col(GroupPermissions::Permission)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GroupPermissions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum GroupPermissions {
    Table,
    Id,
    GroupId,
    Permission,
    CreatedBy,
    CreatedAt,
}
