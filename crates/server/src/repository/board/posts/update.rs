use chrono::Utc;
use entity::board_posts::{
    ActiveModel as PostActiveModel, Entity as PostEntity, Model as PostModel,
};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use uuid::Uuid;

pub async fn repository_update_board_post<C>(
    conn: &C,
    id: Uuid,
    title: Option<String>,
    content: Option<String>,
) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    let post = PostEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardPostNotFound)?;

    let mut active: PostActiveModel = post.into();

    let mut changed = false;
    if let Some(title) = title {
        active.title = Set(title);
        changed = true;
    }
    if let Some(content) = content {
        active.content = Set(content);
        changed = true;
    }
    if changed {
        active.edited_at = Set(Some(Utc::now()));
    }

    let updated = active.update(conn).await?;
    Ok(updated)
}
