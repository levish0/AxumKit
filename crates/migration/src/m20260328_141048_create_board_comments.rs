use crate::m20250825_033643_actors::Actors;
use crate::m20260328_141047_create_board_posts::BoardPosts;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BoardComments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BoardComments::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(BoardComments::PostId).uuid().not_null())
                    .col(ColumnDef::new(BoardComments::ActorId).uuid().not_null())
                    // NULL = top-level comment, Some = reply. Depth is capped at 2 in the
                    // service layer: a reply's parent_comment_id is always a top-level comment.
                    .col(ColumnDef::new(BoardComments::ParentCommentId).uuid().null())
                    .col(ColumnDef::new(BoardComments::Content).text().not_null())
                    .col(
                        ColumnDef::new(BoardComments::ReplyCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(BoardComments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(BoardComments::EditedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_board_comments_post_id")
                            .from(BoardComments::Table, BoardComments::PostId)
                            .to(BoardPosts::Table, BoardPosts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_board_comments_actor_id")
                            .from(BoardComments::Table, BoardComments::ActorId)
                            .to(Actors::Table, Actors::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_board_comments_parent_comment_id")
                            .from(BoardComments::Table, BoardComments::ParentCommentId)
                            .to(BoardComments::Table, BoardComments::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_board_comments_post_parent_created")
                    .table(BoardComments::Table)
                    // Serves both "top-level comments for a post" (parent_comment_id IS NULL)
                    // and "replies under a comment" (parent_comment_id = X), oldest-first.
                    .col(BoardComments::PostId)
                    .col(BoardComments::ParentCommentId)
                    .col(BoardComments::CreatedAt)
                    .col(BoardComments::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_board_comments_actor_id")
                    .table(BoardComments::Table)
                    .col(BoardComments::ActorId)
                    .to_owned(),
            )
            .await?;

        // The self-FK column is not covered by the post_id-leading composite above, and
        // Postgres does not auto-index FK columns. Index it so the cascade delete of a
        // top-level comment's replies and the FK referential checks stay index-driven.
        manager
            .create_index(
                Index::create()
                    .name("idx_board_comments_parent_comment_id")
                    .table(BoardComments::Table)
                    .col(BoardComments::ParentCommentId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BoardComments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum BoardComments {
    Table,
    Id,
    PostId,
    ActorId,
    ParentCommentId,
    Content,
    ReplyCount,
    CreatedAt,
    EditedAt,
}
