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

        // Create index on handle column (optimize login/search performance)
        manager
            .create_index(
                Index::create()
                    .name("idx_users_handle")
                    .table(Users::Table)
                    .col(Users::Handle)
                    .to_owned(),
            )
            .await?;

        // Case-insensitive unique email. The app normalizes email to lowercase at
        // the repository boundary (normalize_email) and looks up via lower(email),
        // so this functional unique index both enforces uniqueness and serves those
        // lookups (replacing the plain idx_users_email). Defense-in-depth: even a
        // write that bypassed normalization could not create a case-variant duplicate.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS users_email_lower_key \
                 ON users (lower(email));",
            )
            .await?;

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
    VerifiedAt,
    ProfileImage,
    BannerImage,
    // TOTP 2FA
    TotpSecret,
    TotpEnabledAt,
    TotpBackupCodes,
    CreatedAt,
}
