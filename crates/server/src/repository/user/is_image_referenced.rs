use crate::repository::common::repository_query_exists;
use entity::users::{Column as UserColumn, Entity as UserEntity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter};

/// Returns whether a content-addressed user image asset is still referenced by any
/// user's profile or banner image.
///
/// User images are keyed by content hash, so identical uploads share one R2 object
/// across users. Used to guard deletes so a shared image is never removed while another
/// user (or the same user's other image slot) still references it.
pub async fn repository_is_user_image_referenced<C>(
    conn: &C,
    storage_key: &str,
) -> Result<bool, Errors>
where
    C: ConnectionTrait,
{
    repository_query_exists(
        conn,
        UserEntity::find().filter(
            UserColumn::ProfileImage
                .eq(storage_key)
                .or(UserColumn::BannerImage.eq(storage_key)),
        ),
    )
    .await
}
