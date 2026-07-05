use entity::known_devices::{Column, Entity, Model};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Look up a trusted device for `(user_id, device_hash)`.
pub async fn repository_find_known_device<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    device_hash: &str,
) -> Result<Option<Model>, Errors> {
    Ok(Entity::find()
        .filter(Column::UserId.eq(user_id))
        .filter(Column::DeviceHash.eq(device_hash))
        .one(conn)
        .await?)
}
