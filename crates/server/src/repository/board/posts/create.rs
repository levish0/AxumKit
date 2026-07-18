use entity::board_posts::{ActiveModel as PostActiveModel, Model as PostModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

pub async fn repository_create_board_post<C>(
    conn: &C,
    board_id: Uuid,
    actor_id: Uuid,
    title: String,
    content: String,
) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    let new_post = PostActiveModel {
        id: Default::default(),
        board_id: Set(board_id),
        actor_id: Set(actor_id),
        title: Set(title),
        content: Set(content),
        pinned_position: Set(None),
        is_locked: Set(false),
        view_count: Set(0),
        comment_count: Set(0),
        created_at: Default::default(),
        edited_at: Set(None),
    };

    let post = new_post.insert(conn).await?;
    Ok(post)
}
