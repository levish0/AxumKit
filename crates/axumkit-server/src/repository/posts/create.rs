use axumkit_entity::posts::{ActiveModel as PostActiveModel, Model as PostModel};
use axumkit_errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

pub async fn repository_create_post<C>(
    conn: &C,
    author_id: Uuid,
    title: String,
    storage_key: String,
) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    let new_post = PostActiveModel {
        id: Default::default(),
        author_id: Set(author_id),
        title: Set(title),
        storage_key: Set(storage_key),
        created_at: Default::default(),
        updated_at: Default::default(),
    };

    let post = new_post.insert(conn).await?;

    Ok(post)
}
