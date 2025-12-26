use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserRefreshTokens::Table)
                    .add_column(ColumnDef::new(UserRefreshTokens::IpAddress).text().null())
                    .add_column(ColumnDef::new(UserRefreshTokens::UserAgent).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserRefreshTokens::Table)
                    .drop_column(UserRefreshTokens::IpAddress)
                    .drop_column(UserRefreshTokens::UserAgent)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum UserRefreshTokens {
    Table,
    IpAddress,
    UserAgent,
}
