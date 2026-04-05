use sea_orm_migration::prelude::*;
use strum::EnumIter;

#[derive(DeriveIden, EnumIter)]
pub enum Role {
    #[sea_orm(iden = "role")]
    Table,
    #[sea_orm(iden = "mod")]
    Mod,
    #[sea_orm(iden = "admin")]
    Admin,
}
