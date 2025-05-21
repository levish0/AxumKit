use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\";")
            .await?;

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
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(string_len(Users::Name, 20).not_null())
                    .col(string_len(Users::Handle, 20).not_null().unique_key())
                    .col(string_len(Users::Email, 254).not_null().unique_key())
                    .col(ColumnDef::new(Users::Password).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_users_handle")
                    .table(Users::Table)
                    .col(Users::Handle)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_users_handle")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Name,
    Handle,
    Email,
    Password,
}
