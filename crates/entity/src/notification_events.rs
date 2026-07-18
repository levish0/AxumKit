use sea_orm::prelude::IpNetwork;
use sea_orm::prelude::*;
use uuid::Uuid;

use super::actors::Entity as ActorsEntity;
use super::board_comments::Entity as BoardCommentsEntity;
use super::board_posts::Entity as BoardPostsEntity;
use super::boards::Entity as BoardsEntity;
use super::common::{NotificationTargetKind, NotificationType};
use super::notification_deliveries::Entity as NotificationDeliveriesEntity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(nullable)]
    pub actor_id: Option<Uuid>,
    #[sea_orm(nullable)]
    pub actor_ip: Option<IpNetwork>,
    #[sea_orm(not_null)]
    pub notification_type: NotificationType,
    #[sea_orm(column_type = "Text", not_null)]
    pub action: String,
    #[sea_orm(not_null)]
    pub target_kind: NotificationTargetKind,
    #[sea_orm(nullable)]
    pub board_id: Option<Uuid>,
    #[sea_orm(nullable)]
    pub post_id: Option<Uuid>,
    #[sea_orm(nullable)]
    pub comment_id: Option<Uuid>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub additional_data: Option<Json>,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "ActorsEntity",
        from = "Column::ActorId",
        to = "super::actors::Column::Id",
        on_delete = "SetNull"
    )]
    Actor,
    #[sea_orm(
        belongs_to = "BoardsEntity",
        from = "Column::BoardId",
        to = "super::boards::Column::Id",
        on_delete = "Cascade"
    )]
    Board,
    #[sea_orm(
        belongs_to = "BoardPostsEntity",
        from = "Column::PostId",
        to = "super::board_posts::Column::Id",
        on_delete = "Cascade"
    )]
    BoardPost,
    #[sea_orm(
        belongs_to = "BoardCommentsEntity",
        from = "Column::CommentId",
        to = "super::board_comments::Column::Id",
        on_delete = "Cascade"
    )]
    BoardComment,
    #[sea_orm(has_many = "NotificationDeliveriesEntity")]
    Deliveries,
}

impl Related<ActorsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Actor.def()
    }
}

impl Related<BoardsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Board.def()
    }
}

impl Related<BoardPostsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::BoardPost.def()
    }
}

impl Related<BoardCommentsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::BoardComment.def()
    }
}

impl Related<NotificationDeliveriesEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Deliveries.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
