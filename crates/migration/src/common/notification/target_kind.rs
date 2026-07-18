use sea_orm_migration::prelude::*;
use strum::EnumIter;

#[derive(DeriveIden, EnumIter)]
pub enum NotificationTargetKind {
    #[sea_orm(iden = "notification_target_kind")]
    Table,
    #[sea_orm(iden = "none")]
    None,
    #[sea_orm(iden = "board_post")]
    BoardPost,
    #[sea_orm(iden = "board_comment")]
    BoardComment,
}
