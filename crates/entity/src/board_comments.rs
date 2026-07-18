use sea_orm::prelude::*;
use uuid::Uuid;

use super::actors::Entity as ActorsEntity;
use super::board_posts::Entity as BoardPostsEntity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "board_comments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(not_null)]
    pub post_id: Uuid,
    #[sea_orm(not_null)]
    pub actor_id: Uuid,
    /// NULL = top-level comment, Some = reply. Depth is capped at 2 in the service
    /// layer, so a reply's `parent_comment_id` always points at a top-level comment.
    #[sea_orm(nullable)]
    pub parent_comment_id: Option<Uuid>,
    #[sea_orm(column_type = "Text", not_null)]
    pub content: String,
    #[sea_orm(not_null, default_value = "0")]
    pub reply_count: i32,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub edited_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "BoardPostsEntity",
        from = "Column::PostId",
        to = "super::board_posts::Column::Id",
        on_delete = "Cascade"
    )]
    Post,
    #[sea_orm(
        belongs_to = "ActorsEntity",
        from = "Column::ActorId",
        to = "super::actors::Column::Id",
        on_delete = "Restrict"
    )]
    Actor,
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentCommentId",
        to = "Column::Id",
        on_delete = "Cascade"
    )]
    ParentComment,
}

impl Related<BoardPostsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Post.def()
    }
}

impl Related<ActorsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Actor.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
