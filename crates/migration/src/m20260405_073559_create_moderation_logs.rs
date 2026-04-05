use crate::common::moderation::ModerationResourceType;
use crate::m20250825_033639_users::Users;
use sea_orm_migration::prelude::*;
use strum::IntoEnumIterator;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ModerationLogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ModerationLogs::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(ModerationLogs::Action).text().not_null())
                    .col(ColumnDef::new(ModerationLogs::ActorId).uuid().null())
                    .col(
                        ColumnDef::new(ModerationLogs::ResourceType)
                            .enumeration(
                                ModerationResourceType::Table,
                                ModerationResourceType::iter()
                                    .filter(|p| !matches!(p, ModerationResourceType::Table))
                                    .collect::<Vec<_>>(),
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(ModerationLogs::ResourceId).uuid().null())
                    .col(ColumnDef::new(ModerationLogs::Reason).text().not_null())
                    .col(
                        ColumnDef::new(ModerationLogs::Metadata)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ModerationLogs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_moderation_logs_actor")
                            .from(ModerationLogs::Table, ModerationLogs::ActorId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_moderation_logs_resource")
                    .table(ModerationLogs::Table)
                    .col(ModerationLogs::ResourceType)
                    .col(ModerationLogs::ResourceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_moderation_logs_actor")
                    .table(ModerationLogs::Table)
                    .col(ModerationLogs::ActorId)
                    .cond_where(Expr::col(ModerationLogs::ActorId).is_not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_moderation_logs_action")
                    .table(ModerationLogs::Table)
                    .col(ModerationLogs::Action)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ModerationLogs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum ModerationLogs {
    Table,
    Id,
    Action,
    ActorId,
    ResourceType,
    ResourceId,
    Reason,
    Metadata,
    CreatedAt,
}
