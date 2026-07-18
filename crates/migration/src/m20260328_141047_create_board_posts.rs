use crate::m20250825_033643_actors::Actors;
use crate::m20260328_141037_create_boards::Boards;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BoardPosts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BoardPosts::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(BoardPosts::BoardId).uuid().not_null())
                    .col(ColumnDef::new(BoardPosts::ActorId).uuid().not_null())
                    .col(ColumnDef::new(BoardPosts::Title).text().not_null())
                    .col(ColumnDef::new(BoardPosts::Content).text().not_null())
                    // Pin slot and pin flag in one column: NULL is unpinned, a
                    // value is the post's rank among the board's pins (lower is
                    // higher). Two columns would let `is_pinned` and a position
                    // drift out of sync; this cannot. Deliberately not unique —
                    // see the repository's pin write for why ties are allowed.
                    .col(ColumnDef::new(BoardPosts::PinnedPosition).integer().null())
                    .col(
                        ColumnDef::new(BoardPosts::IsLocked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(BoardPosts::ViewCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(BoardPosts::CommentCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(BoardPosts::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(BoardPosts::EditedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_board_posts_board_id")
                            .from(BoardPosts::Table, BoardPosts::BoardId)
                            .to(Boards::Table, Boards::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_board_posts_actor_id")
                            .from(BoardPosts::Table, BoardPosts::ActorId)
                            .to(Actors::Table, Actors::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_board_posts_board_pinned_created")
                    .table(BoardPosts::Table)
                    // Board listing splits on `pinned_position IS NULL`: pins are
                    // read whole and ordered by position, the rest paged
                    // newest-first. Both are an equality match on the first two
                    // columns, so this one index serves each as a prefix.
                    .col(BoardPosts::BoardId)
                    .col(BoardPosts::PinnedPosition)
                    .col((BoardPosts::CreatedAt, IndexOrder::Desc))
                    .col((BoardPosts::Id, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_board_posts_actor_id")
                    .table(BoardPosts::Table)
                    .col(BoardPosts::ActorId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BoardPosts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum BoardPosts {
    Table,
    Id,
    BoardId,
    ActorId,
    Title,
    Content,
    PinnedPosition,
    IsLocked,
    ViewCount,
    CommentCount,
    CreatedAt,
    EditedAt,
}
