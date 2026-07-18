use sea_orm_migration::prelude::*;
use strum::EnumIter;

#[derive(DeriveIden, EnumIter)]
pub enum ModerationResourceType {
    #[sea_orm(iden = "moderation_resource_type")]
    Table,
    #[sea_orm(iden = "user")]
    User,
    #[sea_orm(iden = "system")]
    System,
    #[sea_orm(iden = "acl_group")]
    Group,
    #[sea_orm(iden = "board")]
    Board,
    #[sea_orm(iden = "board_post")]
    BoardPost,
}
