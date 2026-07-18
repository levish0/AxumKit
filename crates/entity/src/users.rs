use sea_orm::prelude::*;
use uuid::Uuid;

use super::actors::Entity as ActorsEntity;
use super::user_oauth_connections::Entity as UserOAuthConnectionsEntity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text", not_null)]
    pub display_name: String,
    #[sea_orm(column_type = "Text", not_null, unique)]
    pub handle: String, // Unique
    #[sea_orm(column_type = "Text", nullable)]
    pub bio: Option<String>,
    // Case-insensitive uniqueness is enforced by a lower(email) functional unique
    // index (see the users migration), not a plain column unique constraint.
    #[sea_orm(string_len = 254, not_null)]
    pub email: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub password: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub profile_image: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub banner_image: Option<String>,
    // TOTP 2FA
    /// TOTP secret — AES-256-GCM encrypted at rest, stored as base64(nonce ‖ ciphertext)
    #[sea_orm(column_type = "Text", nullable)]
    pub totp_secret: Option<String>,
    /// TOTP enabled timestamp (None = disabled)
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub totp_enabled_at: Option<DateTimeUtc>,
    /// TOTP backup codes (10, stored as Blake3 keyed-hash)
    #[sea_orm(nullable)]
    pub totp_backup_codes: Option<Vec<String>>,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
    /// Soft-delete timestamp (None = active, Some = deactivated)
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub deleted_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "ActorsEntity")]
    Actors,
    #[sea_orm(has_many = "UserOAuthConnectionsEntity")]
    OAuthConnections,
}

impl Related<ActorsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Actors.def()
    }
}

impl Related<UserOAuthConnectionsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::OAuthConnections.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
