use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserOauthAccounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserOauthAccounts::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserOauthAccounts::UserId).uuid().not_null())
                    .col(string_len(UserOauthAccounts::Provider, 50).not_null())
                    .col(
                        ColumnDef::new(UserOauthAccounts::ProviderId)
                            .text()
                            .not_null(),
                    )
                    .col(string_len(UserOauthAccounts::Email, 254).not_null())
                    .col(ColumnDef::new(UserOauthAccounts::AccessToken).text().null())
                    .col(
                        ColumnDef::new(UserOauthAccounts::RefreshToken)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserOauthAccounts::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserOauthAccounts::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserOauthAccounts::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserOauthAccounts::Table, UserOauthAccounts::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_user_provider_providerid_unique")
                            .table(UserOauthAccounts::Table)
                            .col(UserOauthAccounts::UserId)
                            .col(UserOauthAccounts::Provider)
                            .col(UserOauthAccounts::ProviderId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserOauthAccounts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserOauthAccounts {
    Table,
    Id,
    UserId,
    Provider,
    ProviderId,
    Email,
    AccessToken,
    RefreshToken,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
