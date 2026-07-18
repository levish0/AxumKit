use sea_orm_migration::prelude::*;
use strum::EnumIter;

#[derive(DeriveIden, EnumIter)]
pub enum ActorKind {
    #[sea_orm(iden = "actor_kind")]
    Table,
    #[sea_orm(iden = "user")]
    User,
    #[sea_orm(iden = "anonymous")]
    Anonymous,
    #[sea_orm(iden = "system")]
    System,
}
