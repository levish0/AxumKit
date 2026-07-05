use sea_orm::prelude::*;
use uuid::Uuid;

use super::users::Entity as UsersEntity;

/// Trusted-device registry for new-device login verification (OWASP ASVS 6.3.5).
///
/// A row exists once a device has been verified for a user; a matching `(user_id, device_hash)`
/// on a later login skips the email challenge. `device_hash` is the blake3 hash of the raw
/// device-cookie token (the raw token lives only in the browser cookie).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "known_devices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    #[sea_orm(column_type = "Text", not_null)]
    pub device_hash: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub user_agent: Option<String>,
    #[sea_orm(nullable)]
    pub last_ip: Option<IpNetwork>,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub first_seen: DateTimeUtc,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub last_seen: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "UsersEntity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<UsersEntity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
