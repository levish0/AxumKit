use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Note: uuidv7() is a built-in function in PostgreSQL 18+, no extension needed

        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(Users::Handle).text().not_null().unique_key())
                    .col(ColumnDef::new(Users::DisplayName).text().not_null())
                    .col(ColumnDef::new(Users::Bio).text().null())
                    .col(string_len(Users::Email, 254).not_null().unique_key())
                    .col(ColumnDef::new(Users::Password).text().null())
                    .col(
                        ColumnDef::new(Users::VerifiedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(Users::ProfileImage).text().null())
                    .col(ColumnDef::new(Users::BannerImage).text().null())
                    // TOTP 2FA
                    .col(ColumnDef::new(Users::TotpSecret).text().null())
                    .col(
                        ColumnDef::new(Users::TotpEnabledAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Users::TotpBackupCodes)
                            .array(ColumnType::Text)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .to_owned(),
            )
            .await?;

        // handle 컬럼 인덱스 생성 (로그인/검색 성능 최적화)
        manager
            .create_index(
                Index::create()
                    .name("idx_users_handle")
                    .table(Users::Table)
                    .col(Users::Handle)
                    .to_owned(),
            )
            .await?;

        // email 컬럼 인덱스 생성 (로그인/검색 성능 최적화)
        manager
            .create_index(
                Index::create()
                    .name("idx_users_email")
                    .table(Users::Table)
                    .col(Users::Email)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Users {
    Table,
    Id,
    Handle,
    DisplayName,
    Bio,
    Email,
    Password,
    VerifiedAt,
    ProfileImage,
    BannerImage,
    // TOTP 2FA
    TotpSecret,
    TotpEnabledAt,
    TotpBackupCodes,
    CreatedAt,
}
