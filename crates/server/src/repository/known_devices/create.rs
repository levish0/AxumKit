use entity::known_devices::{ActiveModel, Model};
use errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

/// Register a newly-verified device for a user.
pub async fn repository_register_known_device<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    device_hash: String,
    user_agent: Option<String>,
    last_ip: Option<IpNetwork>,
) -> Result<Model, Errors> {
    let model = ActiveModel {
        id: Default::default(),
        user_id: Set(user_id),
        device_hash: Set(device_hash),
        user_agent: Set(user_agent),
        last_ip: Set(last_ip),
        first_seen: Default::default(), // DB default now()
        last_seen: Default::default(),  // DB default now()
    };
    Ok(model.insert(conn).await?)
}
