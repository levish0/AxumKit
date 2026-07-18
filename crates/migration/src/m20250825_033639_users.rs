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
                    // Uniqueness is enforced case-insensitively by a lower(email)
                    // functional unique index (created below), not a plain unique
                    // constraint, so Foo@x.com and foo@x.com cannot both exist.
                    .col(string_len(Users::Email, 254).not_null())
                    .col(ColumnDef::new(Users::Password).text().null())
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
                    // Soft delete: NULL = active, timestamp = deactivated (account deleted)
                    .col(
                        ColumnDef::new(Users::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Case-insensitive unique email. The app normalizes email to lowercase at
        // the repository boundary (normalize_email) and looks up via lower(email),
        // so this functional unique index both enforces uniqueness and serves those
        // lookups. Defense-in-depth: even a write that bypassed normalization could
        // not create a case-variant duplicate.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS users_email_lower_key \
                 ON users (lower(email));",
            )
            .await?;

        // profile image orphan lookup optimization
        manager
            .create_index(
                Index::create()
                    .name("idx_users_profile_image")
                    .table(Users::Table)
                    .col(Users::ProfileImage)
                    .cond_where(Expr::col(Users::ProfileImage).is_not_null())
                    .to_owned(),
            )
            .await?;

        // banner image orphan lookup optimization
        manager
            .create_index(
                Index::create()
                    .name("idx_users_banner_image")
                    .table(Users::Table)
                    .col(Users::BannerImage)
                    .cond_where(Expr::col(Users::BannerImage).is_not_null())
                    .to_owned(),
            )
            .await?;

        // Note: idx_users_handle removed - redundant with the UNIQUE constraint
        // (PostgreSQL auto-creates users_handle_key). Email uniqueness/lookups are
        // served by the users_email_lower_key functional unique index above.
        Ok(())
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
    ProfileImage,
    BannerImage,
    // TOTP 2FA
    TotpSecret,
    TotpEnabledAt,
    TotpBackupCodes,
    CreatedAt,
    DeletedAt,
}
