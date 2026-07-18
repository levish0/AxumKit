use entity::board_comments::{ActiveModel as CommentActiveModel, Model as CommentModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

pub async fn repository_create_board_comment<C>(
    conn: &C,
    post_id: Uuid,
    actor_id: Uuid,
    parent_comment_id: Option<Uuid>,
    content: String,
) -> Result<CommentModel, Errors>
where
    C: ConnectionTrait,
{
    let new_comment = CommentActiveModel {
        id: Default::default(),
        post_id: Set(post_id),
        actor_id: Set(actor_id),
        parent_comment_id: Set(parent_comment_id),
        content: Set(content),
        reply_count: Set(0),
        created_at: Default::default(),
        edited_at: Set(None),
    };

    let comment = new_comment.insert(conn).await?;
    Ok(comment)
}
