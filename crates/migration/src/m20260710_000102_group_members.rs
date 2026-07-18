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
                    .table(GroupMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GroupMembers::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(GroupMembers::GroupId).uuid().not_null())
                    .col(ColumnDef::new(GroupMembers::UserId).uuid().null())
                    .col(ColumnDef::new(GroupMembers::IpAddress).inet().null())
                    .col(ColumnDef::new(GroupMembers::Reason).text().null())
                    .col(
                        ColumnDef::new(GroupMembers::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    ) // NULL = permanent membership
                    .col(ColumnDef::new(GroupMembers::CreatedBy).uuid().null())
                    .col(
                        ColumnDef::new(GroupMembers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_members_group")
                            .from(GroupMembers::Table, GroupMembers::GroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_members_user")
                            .from(GroupMembers::Table, GroupMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_members_created_by")
                            .from(GroupMembers::Table, GroupMembers::CreatedBy)
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
                "ALTER TABLE group_members \
                 ADD CONSTRAINT chk_group_members_subject \
                 CHECK (num_nonnulls(user_id, ip_address) = 1)",
            )
            .await?;

        // One membership per subject per group (NULLs never collide, so the
        // two partial-shaped unique indexes coexist).
        manager
            .create_index(
                Index::create()
                    .name("idx_group_members_group_user")
                    .table(GroupMembers::Table)
                    .col(GroupMembers::GroupId)
                    .col(GroupMembers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_group_members_group_ip")
                    .table(GroupMembers::Table)
                    .col(GroupMembers::GroupId)
                    .col(GroupMembers::IpAddress)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Membership lookup for an authenticated subject.
        manager
            .create_index(
                Index::create()
                    .name("idx_group_members_user")
                    .table(GroupMembers::Table)
                    .col(GroupMembers::UserId)
                    .to_owned(),
            )
            .await?;

        // Support INET containment lookups for IP membership checks.
        manager
            .create_index(
                Index::create()
                    .name("idx_group_members_ip_spgist")
                    .table(GroupMembers::Table)
                    .col(GroupMembers::IpAddress)
                    .index_type(IndexType::Custom(Alias::new("spgist").into()))
                    .to_owned(),
            )
            .await?;

        // Speeds up listing and cleanup of active or expired memberships.
        manager
            .create_index(
                Index::create()
                    .name("idx_group_members_expires_at")
                    .table(GroupMembers::Table)
                    .col(GroupMembers::ExpiresAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GroupMembers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum GroupMembers {
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
