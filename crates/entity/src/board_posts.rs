use sea_orm::prelude::*;
use uuid::Uuid;

use super::actors::Entity as ActorsEntity;
use super::boards::Entity as BoardsEntity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "board_posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(not_null)]
    pub board_id: Uuid,
    #[sea_orm(not_null)]
    pub actor_id: Uuid,
    #[sea_orm(column_type = "Text", not_null)]
    pub title: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub content: String,
    /// Rank among the board's pinned posts, lower first; `None` is unpinned.
    /// A relative sort key, not a dense sequence — unpinning leaves gaps and
    /// concurrent pins may tie, so reads break ties on `id`.
    #[sea_orm(nullable)]
    pub pinned_position: Option<i32>,
    #[sea_orm(not_null, default_value = "false")]
    pub is_locked: bool,
    #[sea_orm(not_null, default_value = "0")]
    pub view_count: i32,
    #[sea_orm(not_null, default_value = "0")]
    pub comment_count: i32,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub edited_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "BoardsEntity",
        from = "Column::BoardId",
        to = "super::boards::Column::Id",
        on_delete = "Cascade"
    )]
    Board,
    #[sea_orm(
        belongs_to = "ActorsEntity",
        from = "Column::ActorId",
        to = "super::actors::Column::Id",
        on_delete = "Restrict"
    )]
    Actor,
}

impl Related<BoardsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Board.def()
    }
}

impl Related<ActorsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Actor.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
