use crate::JobState;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(100))")]
    pub kind: String,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub queue: String,
    #[sea_orm(column_type = "String(StringLen::N(128))")]
    pub schedule_key: Option<String>,
    #[sea_orm(column_type = "JsonBinary")]
    pub payload: Json,
    pub request_id: Uuid,
    pub state: JobState,
    pub attempts: i32,
    pub max_attempts: i32,
    pub available_at: DateTimeUtc,
    pub claim_token: Option<Uuid>,
    pub lease_until: Option<DateTimeUtc>,
    pub deadline_at: Option<DateTimeUtc>,
    #[sea_orm(column_type = "String(StringLen::N(100))")]
    pub last_error: Option<String>,
    pub created_at: DateTimeUtc,
    pub finished_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
