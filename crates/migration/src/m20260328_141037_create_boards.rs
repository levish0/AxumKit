use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{OnConflict, Query};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Boards::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Boards::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(Boards::Slug).text().not_null().unique_key())
                    .col(ColumnDef::new(Boards::Name).text().not_null())
                    .col(ColumnDef::new(Boards::Description).text().null())
                    .col(
                        ColumnDef::new(Boards::Order)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Boards::IsDisabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Boards::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(Boards::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_boards_order")
                    .table(Boards::Table)
                    .col(Boards::Order)
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Boards::Table)
                    .columns([
                        Boards::Slug,
                        Boards::Name,
                        Boards::Description,
                        Boards::Order,
                    ])
                    .values_panic([
                        Expr::value("notice"),
                        Expr::value("Announcements"),
                        Expr::value("Official announcements from the operators."),
                        Expr::value(1),
                    ])
                    .values_panic([
                        Expr::value("general"),
                        Expr::value("General"),
                        Expr::value("Open discussion about anything."),
                        Expr::value(2),
                    ])
                    .values_panic([
                        Expr::value("qna"),
                        Expr::value("Q&A"),
                        Expr::value("Ask questions and get help."),
                        Expr::value(3),
                    ])
                    .on_conflict(OnConflict::column(Boards::Slug).do_nothing().to_owned())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Boards::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Boards {
    Table,
    Id,
    Slug,
    Name,
    Description,
    Order,
    IsDisabled,
    CreatedAt,
    UpdatedAt,
}
