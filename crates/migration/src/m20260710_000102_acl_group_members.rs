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
                    .table(AclGroupMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AclGroupMembers::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(AclGroupMembers::GroupId).uuid().not_null())
                    .col(ColumnDef::new(AclGroupMembers::UserId).uuid().null())
                    .col(ColumnDef::new(AclGroupMembers::IpAddress).inet().null())
                    .col(ColumnDef::new(AclGroupMembers::Reason).text().null())
                    .col(
                        ColumnDef::new(AclGroupMembers::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    ) // NULL = permanent membership
                    .col(ColumnDef::new(AclGroupMembers::CreatedBy).uuid().null())
                    .col(
                        ColumnDef::new(AclGroupMembers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_acl_group_members_group")
                            .from(AclGroupMembers::Table, AclGroupMembers::GroupId)
                            .to(AclGroups::Table, AclGroups::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_acl_group_members_user")
                            .from(AclGroupMembers::Table, AclGroupMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_acl_group_members_created_by")
                            .from(AclGroupMembers::Table, AclGroupMembers::CreatedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // A member row is exactly one subject: a user or an IP, never both/neither.
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE acl_group_members \
                 ADD CONSTRAINT chk_acl_group_members_subject \
                 CHECK (num_nonnulls(user_id, ip_address) = 1)",
            )
            .await?;

        // One membership per subject per group (NULLs never collide, so the
        // two partial-shaped unique indexes coexist).
        manager
            .create_index(
                Index::create()
                    .name("idx_acl_group_members_group_user")
                    .table(AclGroupMembers::Table)
                    .col(AclGroupMembers::GroupId)
                    .col(AclGroupMembers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_acl_group_members_group_ip")
                    .table(AclGroupMembers::Table)
                    .col(AclGroupMembers::GroupId)
                    .col(AclGroupMembers::IpAddress)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Membership lookup for an authenticated subject.
        manager
            .create_index(
                Index::create()
                    .name("idx_acl_group_members_user")
                    .table(AclGroupMembers::Table)
                    .col(AclGroupMembers::UserId)
                    .to_owned(),
            )
            .await?;

        // Support INET containment lookups for IP membership checks.
        manager
            .create_index(
                Index::create()
                    .name("idx_acl_group_members_ip_spgist")
                    .table(AclGroupMembers::Table)
                    .col(AclGroupMembers::IpAddress)
                    .index_type(IndexType::Custom(Alias::new("spgist").into()))
                    .to_owned(),
            )
            .await?;

        // Speeds up listing and cleanup of active or expired memberships.
        manager
            .create_index(
                Index::create()
                    .name("idx_acl_group_members_expires_at")
                    .table(AclGroupMembers::Table)
                    .col(AclGroupMembers::ExpiresAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AclGroupMembers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum AclGroupMembers {
    Table,
    Id,
    GroupId,
    UserId,
    IpAddress,
    Reason,
    ExpiresAt,
    CreatedBy,
    CreatedAt,
}
