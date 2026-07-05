use chrono::Utc;
use entity::known_devices::{ActiveModel, Model};
use errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

/// Update a known device's `last_seen` / `last_ip` after a recognized login.
pub async fn repository_touch_known_device<C: ConnectionTrait>(
    conn: &C,
    device: Model,
    last_ip: Option<IpNetwork>,
) -> Result<(), Errors> {
    let mut active: ActiveModel = device.into();
    active.last_seen = Set(Utc::now());
    active.last_ip = Set(last_ip);
    active.update(conn).await?;
    Ok(())
}
