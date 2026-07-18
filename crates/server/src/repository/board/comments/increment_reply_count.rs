use entity::board_comments::{Column as CommentColumn, Entity as CommentEntity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter};
use uuid::Uuid;

/// Increment a top-level comment's `reply_count` (the "N replies" counter).
pub async fn repository_increment_comment_reply_count<C>(
    conn: &C,
    comment_id: Uuid,
) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    CommentEntity::update_many()
        .filter(CommentColumn::Id.eq(comment_id))
        .col_expr(
            CommentColumn::ReplyCount,
            sea_orm::sea_query::Expr::col(CommentColumn::ReplyCount).add(1),
        )
        .exec(conn)
        .await?;

    Ok(())
}
