use entity::board_posts::{Column as PostColumn, Entity as PostEntity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter};
use uuid::Uuid;

pub async fn repository_increment_post_comment_count<C>(
    conn: &C,
    post_id: Uuid,
) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    PostEntity::update_many()
        .filter(PostColumn::Id.eq(post_id))
        .col_expr(
            PostColumn::CommentCount,
            sea_orm::sea_query::Expr::col(PostColumn::CommentCount).add(1),
        )
        .exec(conn)
        .await?;

    Ok(())
}
