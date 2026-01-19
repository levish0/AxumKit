use crate::common::action::ActionResourceType;
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
                    .table(ActionLogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ActionLogs::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(ActionLogs::Action).text().not_null())
                    .col(ColumnDef::new(ActionLogs::ActorId).uuid().null())
                    .col(ColumnDef::new(ActionLogs::ActorIp).inet().null())
                    .col(
                        ColumnDef::new(ActionLogs::ResourceType)
                            .enumeration(
                                ActionResourceType::Table,
                                ActionResourceType::iter()
                                    .filter(|p| !matches!(p, ActionResourceType::Table))
                                    .collect::<Vec<_>>(),
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(ActionLogs::ResourceId).uuid().null())
                    .col(ColumnDef::new(ActionLogs::Summary).text().not_null())
                    .col(ColumnDef::new(ActionLogs::Metadata).json_binary().null())
                    .col(
                        ColumnDef::new(ActionLogs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_action_logs_actor")
                            .from(ActionLogs::Table, ActionLogs::ActorId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // CHECK constraint: at least one of actor_id or actor_ip must be set
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE action_logs ADD CONSTRAINT chk_action_logs_actor \
                 CHECK (((actor_id IS NOT NULL)::int + (actor_ip IS NOT NULL)::int) >= 1)",
            )
            .await?;

        // Index: User actions (filter by actor_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_action_logs_actor_id")
                    .table(ActionLogs::Table)
                    .col(ActionLogs::ActorId)
                    .col(ActionLogs::Id)
                    .to_owned(),
            )
            .await?;

        // Index: Anonymous contributions (filter by actor_ip)
        // Using SP-GiST for INET type
        manager
            .create_index(
                Index::create()
                    .name("idx_action_logs_actor_ip")
                    .table(ActionLogs::Table)
                    .col(ActionLogs::ActorIp)
                    .index_type(IndexType::Custom(Alias::new("spgist").into()))
                    .to_owned(),
            )
            .await?;

        // Index: Resource lookup (filter by resource_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_action_logs_resource")
                    .table(ActionLogs::Table)
                    .col(ActionLogs::ResourceId)
                    .col(ActionLogs::Id)
                    .to_owned(),
            )
            .await?;

        // Index: Filter by action type
        manager
            .create_index(
                Index::create()
                    .name("idx_action_logs_action")
                    .table(ActionLogs::Table)
                    .col(ActionLogs::Action)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ActionLogs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum ActionLogs {
    Table,
    Id,
    Action,
    ActorId,
    ActorIp,
    ResourceType,
    ResourceId,
    Summary,
    Metadata,
    CreatedAt,
}
