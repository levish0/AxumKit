use crate::common::OAuthProvider;
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
                    .table(UserOAuthConnections::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserOAuthConnections::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(
                        ColumnDef::new(UserOAuthConnections::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserOAuthConnections::Provider)
                            .enumeration(OAuthProvider::Table, OAuthProvider::iter().skip(1))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserOAuthConnections::ProviderUserId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserOAuthConnections::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserOAuthConnections::Table, UserOAuthConnections::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. provider + provider_user_id 유니크 제약조건
        manager
            .create_index(
                Index::create()
                    .name("idx_user_oauth_connections_provider_user_id")
                    .table(UserOAuthConnections::Table)
                    .col(UserOAuthConnections::Provider)
                    .col(UserOAuthConnections::ProviderUserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // 4. user_id + provider 유니크 제약조건 (한 유저당 provider별 하나씩만)
        manager
            .create_index(
                Index::create()
                    .name("idx_user_oauth_connections_user_provider")
                    .table(UserOAuthConnections::Table)
                    .col(UserOAuthConnections::UserId)
                    .col(UserOAuthConnections::Provider)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // 5. user_id 인덱스 (유저별 연동된 제공자 조회용)
        manager
            .create_index(
                Index::create()
                    .name("idx_user_oauth_connections_user_id")
                    .table(UserOAuthConnections::Table)
                    .col(UserOAuthConnections::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserOAuthConnections::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserOAuthConnections {
    #[sea_orm(iden = "user_oauth_connections")]
    Table,
    Id,
    UserId,
    Provider,
    ProviderUserId,
    CreatedAt,
}
