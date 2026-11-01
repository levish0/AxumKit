use crate::AttemptOutcome;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "job_attempts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub job_id: Uuid,
    pub attempt: i32,
    pub started_at: DateTimeUtc,
    pub finished_at: Option<DateTimeUtc>,
    pub outcome: Option<AttemptOutcome>,
    #[sea_orm(column_type = "String(StringLen::N(100))")]
    pub error_code: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
