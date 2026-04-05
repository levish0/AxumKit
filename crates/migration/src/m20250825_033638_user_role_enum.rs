use crate::common::Role;
use crate::extension::postgres::Type;
use sea_orm_migration::prelude::*;
use strum::IntoEnumIterator;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(Role::Table)
                    .values(
                        Role::iter()
                            .filter(|p| !matches!(p, Role::Table))
                            .collect::<Vec<_>>(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_type(Type::drop().if_exists().name(Role::Table).to_owned())
            .await
    }
}
