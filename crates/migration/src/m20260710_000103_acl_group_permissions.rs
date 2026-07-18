use crate::m20250825_033639_users::Users;
use crate::m20260710_000101_acl_groups::AclGroups;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AclGroupPermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AclGroupPermissions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(
                        ColumnDef::new(AclGroupPermissions::GroupId)
                            .uuid()
                            .not_null(),
                    )
                    // Permission codename stored as TEXT (not a PG enum): the
                    // permission set grows with features, so additions must not
                    // require an enum migration.
                    .col(
                        ColumnDef::new(AclGroupPermissions::Permission)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AclGroupPermissions::CreatedBy).uuid().null())
                    .col(
                        ColumnDef::new(AclGroupPermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_acl_group_permissions_group")
                            .from(AclGroupPermissions::Table, AclGroupPermissions::GroupId)
                            .to(AclGroups::Table, AclGroups::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_acl_group_permissions_created_by")
                            .from(AclGroupPermissions::Table, AclGroupPermissions::CreatedBy)
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
                    .name("uq_acl_group_permissions_group_permission")
                    .table(AclGroupPermissions::Table)
                    .col(AclGroupPermissions::GroupId)
                    .col(AclGroupPermissions::Permission)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AclGroupPermissions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum AclGroupPermissions {
    Table,
    Id,
    GroupId,
    Permission,
    CreatedBy,
    CreatedAt,
}
