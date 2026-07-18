use entity::board_comments::{Column as CommentColumn, Entity as CommentEntity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter};
use uuid::Uuid;

/// Decrement a top-level comment's `reply_count` when one of its replies is deleted.
pub async fn repository_decrement_comment_reply_count<C>(
    conn: &C,
    comment_id: Uuid,
) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    // Floor at 0: never decrement below zero, so counter drift can't go negative.
    CommentEntity::update_many()
        .filter(CommentColumn::Id.eq(comment_id))
        .filter(CommentColumn::ReplyCount.gt(0))
        .col_expr(
            CommentColumn::ReplyCount,
            sea_orm::sea_query::Expr::col(CommentColumn::ReplyCount).sub(1),
        )
        .exec(conn)
        .await?;

    Ok(())
}
