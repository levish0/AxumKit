use axumkit_entity::posts::{ActiveModel as PostActiveModel, Model as PostModel};
use axumkit_errors::errors::Errors;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, IntoActiveModel, Set};

pub struct PostUpdateParams {
    pub title: Option<String>,
    pub storage_key: Option<String>,
}

pub async fn repository_update_post<C>(
    conn: &C,
    post: PostModel,
    params: PostUpdateParams,
) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    let mut active_model: PostActiveModel = post.into_active_model();

    if let Some(title) = params.title {
        active_model.title = Set(title);
    }

    if let Some(storage_key) = params.storage_key {
        active_model.storage_key = Set(storage_key);
    }

    active_model.updated_at = Set(Utc::now().into());

    let updated_post = active_model.update(conn).await?;

    Ok(updated_post)
}
