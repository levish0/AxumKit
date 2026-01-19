use sea_orm_migration::prelude::*;
use strum::EnumIter;

#[derive(DeriveIden, EnumIter)]
pub enum ActionResourceType {
    #[sea_orm(iden = "action_resource_type")]
    Table,
    #[sea_orm(iden = "user")]
    User,
    #[sea_orm(iden = "post")]
    Post,
}
