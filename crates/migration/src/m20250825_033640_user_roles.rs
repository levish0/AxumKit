use crate::common::Role;
use crate::m20250825_033639_users::Users;
use sea_orm_migration::prelude::*;
use strum::IntoEnumIterator;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserRoles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserRoles::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(UserRoles::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserRoles::Role)
                            .enumeration(
                                Role::Table,
                                Role::iter()
                                    .filter(|p| !matches!(p, Role::Table))
                                    .collect::<Vec<_>>(),
                            )
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRoles::GrantedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(UserRoles::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_roles_user")
                            .from(UserRoles::Table, UserRoles::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_roles_user_role_unique")
                    .table(UserRoles::Table)
                    .col(UserRoles::UserId)
                    .col(UserRoles::Role)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Reverse lookup: find all users with a specific role
        manager
            .create_index(
                Index::create()
                    .name("idx_user_roles_role")
                    .table(UserRoles::Table)
                    .col(UserRoles::Role)
                    .to_owned(),
            )
            .await?;

        // Expiration cleanup
        manager
            .create_index(
                Index::create()
                    .name("idx_user_roles_expires_at")
                    .table(UserRoles::Table)
                    .col(UserRoles::ExpiresAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserRoles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum UserRoles {
    Table,
    Id,
    UserId,
    Role,
    GrantedAt,
    ExpiresAt,
}
