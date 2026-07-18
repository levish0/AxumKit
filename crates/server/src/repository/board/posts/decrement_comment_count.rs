use entity::board_posts::{Column as PostColumn, Entity as PostEntity};
use errors::errors::Errors;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Decrement a post's `comment_count` by `by` (a top-level comment delete removes
/// the comment plus its cascaded replies, so `by` may be greater than 1).
pub async fn repository_decrement_post_comment_count<C>(
    conn: &C,
    post_id: Uuid,
    by: i32,
) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    // Clamp to 0 in a single statement: even if the stored count has drifted below
    // `by`, the decrement still applies (down to 0) instead of no-op'ing and leaving
    // the counter stuck high.
    PostEntity::update_many()
        .filter(PostColumn::Id.eq(post_id))
        .col_expr(
            PostColumn::CommentCount,
            Expr::cust_with_values("GREATEST(comment_count - $1, 0)", [by]),
        )
        .exec(conn)
        .await?;

    Ok(())
}
