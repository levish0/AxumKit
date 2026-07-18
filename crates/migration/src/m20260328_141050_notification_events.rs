use crate::common::notification::{NotificationTargetKind, NotificationType};
use crate::m20250825_033643_actors::Actors;
use crate::m20260328_141037_create_boards::Boards;
use crate::m20260328_141047_create_board_posts::BoardPosts;
use crate::m20260328_141048_create_board_comments::BoardComments;
use sea_orm_migration::{prelude::*, schema::*};
use strum::IntoEnumIterator;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Pin down the shape target_kind points at: even with multiple nullable
        // FKs, the allowed combinations are defined only here.
        let target_constraint = Cond::any()
            .add(
                Cond::all()
                    .add(Expr::col(NotificationEvents::TargetKind).eq("none"))
                    .add(Expr::col(NotificationEvents::BoardId).is_null())
                    .add(Expr::col(NotificationEvents::PostId).is_null())
                    .add(Expr::col(NotificationEvents::CommentId).is_null()),
            )
            .add(
                Cond::all()
                    .add(Expr::col(NotificationEvents::TargetKind).eq("board_post"))
                    .add(Expr::col(NotificationEvents::BoardId).is_not_null())
                    .add(Expr::col(NotificationEvents::PostId).is_not_null())
                    .add(Expr::col(NotificationEvents::CommentId).is_null()),
            )
            .add(
                Cond::all()
                    .add(Expr::col(NotificationEvents::TargetKind).eq("board_comment"))
                    .add(Expr::col(NotificationEvents::BoardId).is_not_null())
                    .add(Expr::col(NotificationEvents::PostId).is_not_null())
                    .add(Expr::col(NotificationEvents::CommentId).is_not_null()),
            );

        // Pin down the action / notification_type / target_kind combinations —
        // e.g. user_mentioned only ever attaches to board posts and comments.
        let action_constraint = Cond::any()
            .add(
                Cond::all()
                    .add(Expr::col(NotificationEvents::Action).eq("user_mentioned"))
                    .add(Expr::col(NotificationEvents::NotificationType).eq("user"))
                    .add(
                        Cond::any()
                            .add(Expr::col(NotificationEvents::TargetKind).eq("board_post"))
                            .add(Expr::col(NotificationEvents::TargetKind).eq("board_comment")),
                    ),
            )
            .add(
                Cond::all()
                    .add(Expr::col(NotificationEvents::Action).eq("board_comment_created"))
                    .add(Expr::col(NotificationEvents::NotificationType).eq("board"))
                    .add(Expr::col(NotificationEvents::TargetKind).eq("board_comment")),
            )
            .add(
                Cond::all()
                    .add(Expr::col(NotificationEvents::Action).eq("system_announcement"))
                    .add(Expr::col(NotificationEvents::NotificationType).eq("system"))
                    .add(Expr::col(NotificationEvents::TargetKind).eq("none")),
            );

        manager
            .create_table(
                Table::create()
                    .table(NotificationEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NotificationEvents::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(NotificationEvents::ActorId).uuid().null())
                    .col(ColumnDef::new(NotificationEvents::ActorIp).inet().null())
                    .col(
                        ColumnDef::new(NotificationEvents::NotificationType)
                            .enumeration(
                                NotificationType::Table,
                                NotificationType::iter()
                                    .filter(|p| !matches!(p, NotificationType::Table))
                                    .collect::<Vec<_>>(),
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(NotificationEvents::Action).text().not_null())
                    .col(
                        ColumnDef::new(NotificationEvents::TargetKind)
                            .enumeration(
                                NotificationTargetKind::Table,
                                NotificationTargetKind::iter()
                                    .filter(|p| !matches!(p, NotificationTargetKind::Table))
                                    .collect::<Vec<_>>(),
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(NotificationEvents::BoardId).uuid().null())
                    .col(ColumnDef::new(NotificationEvents::PostId).uuid().null())
                    .col(ColumnDef::new(NotificationEvents::CommentId).uuid().null())
                    .col(
                        ColumnDef::new(NotificationEvents::AdditionalData)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        timestamp_with_time_zone(NotificationEvents::CreatedAt)
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    // Only allow the FK combinations valid for each target_kind.
                    .check(("chk_notification_events_target_shape", target_constraint))
                    // Reject events whose action disagrees with the target shape.
                    .check(("chk_notification_events_action_shape", action_constraint))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_events_actor_id")
                            .from(NotificationEvents::Table, NotificationEvents::ActorId)
                            .to(Actors::Table, Actors::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_events_board_id")
                            .from(NotificationEvents::Table, NotificationEvents::BoardId)
                            .to(Boards::Table, Boards::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_events_post_id")
                            .from(NotificationEvents::Table, NotificationEvents::PostId)
                            .to(BoardPosts::Table, BoardPosts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_events_comment_id")
                            .from(NotificationEvents::Table, NotificationEvents::CommentId)
                            .to(BoardComments::Table, BoardComments::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_events_board")
                    .table(NotificationEvents::Table)
                    .col(NotificationEvents::BoardId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_events_post")
                    .table(NotificationEvents::Table)
                    .col(NotificationEvents::PostId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_events_comment")
                    .table(NotificationEvents::Table)
                    .col(NotificationEvents::CommentId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_events_type")
                    .table(NotificationEvents::Table)
                    .col(NotificationEvents::NotificationType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_events_action")
                    .table(NotificationEvents::Table)
                    .col(NotificationEvents::Action)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_events_target_kind")
                    .table(NotificationEvents::Table)
                    .col(NotificationEvents::TargetKind)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_events_actor_ip")
                    .table(NotificationEvents::Table)
                    .col(NotificationEvents::ActorIp)
                    .index_type(IndexType::Custom(Alias::new("spgist").into()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NotificationEvents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum NotificationEvents {
    Table,
    Id,
    ActorId,
    ActorIp,
    NotificationType,
    Action,
    TargetKind,
    BoardId,
    PostId,
    CommentId,
    AdditionalData,
    CreatedAt,
}
