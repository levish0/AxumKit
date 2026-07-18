use crate::common::ActorKind;
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
                    .table(Actors::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Actors::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(
                        ColumnDef::new(Actors::Kind)
                            .enumeration(
                                ActorKind::Table,
                                ActorKind::iter()
                                    .filter(|p| !matches!(p, ActorKind::Table))
                                    .collect::<Vec<_>>(),
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(Actors::UserId).uuid().null())
                    .col(ColumnDef::new(Actors::Ip).inet().null())
                    .col(
                        ColumnDef::new(Actors::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_actors_user")
                            .from(Actors::Table, Actors::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .check(Expr::cust(
                        r#"(
                            (kind = 'user'::actor_kind AND user_id IS NOT NULL AND ip IS NULL)
                            OR (kind = 'anonymous'::actor_kind AND user_id IS NULL AND ip IS NOT NULL)
                            OR (kind = 'system'::actor_kind AND user_id IS NULL AND ip IS NULL)
                        )"#,
                    ))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uq_actors_user_id")
                    .table(Actors::Table)
                    .col(Actors::UserId)
                    .unique()
                    .cond_where(Expr::col(Actors::UserId).is_not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uq_actors_anonymous_ip")
                    .table(Actors::Table)
                    .col(Actors::Ip)
                    .unique()
                    .cond_where(Expr::col(Actors::Kind).eq("anonymous"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_actors_ip")
                    .table(Actors::Table)
                    .col(Actors::Ip)
                    .index_type(IndexType::Custom(Alias::new("spgist").into()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Actors::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Actors {
    Table,
    Id,
    Kind,
    UserId,
    Ip,
    CreatedAt,
}
