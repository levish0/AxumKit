use sea_orm::prelude::*;
use uuid::Uuid;

/// Private-tier authentication audit log (OWASP ASVS V16).
///
/// One row per authentication decision (login success/failure, logout, credential/2FA change),
/// with the actor IP + user-agent. `user_id` is nullable: a failed login on an unknown email has
/// no user. There is intentionally no FK to `users` so the row survives account deletion for
/// forensics. Never publicly exposed; intended for a restricted role and ~90-day retention.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(nullable)]
    pub user_id: Option<Uuid>,
    #[sea_orm(column_type = "Text", not_null)]
    pub event_type: String,
    #[sea_orm(nullable)]
    pub ip: Option<IpNetwork>,
    #[sea_orm(column_type = "Text", nullable)]
    pub user_agent: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub metadata: Option<Json>,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
