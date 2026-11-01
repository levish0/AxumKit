use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub recipient_id: Uuid,
    pub actor_id: Uuid,
    pub organization_id: Uuid,
    pub post_id: Uuid,
    pub comment_id: Uuid,
    pub created_at: DateTimeUtc,
    pub read_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
