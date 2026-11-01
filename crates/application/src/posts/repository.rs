use super::entities::{comment, post, post_attachment};
use crate::{
    files::{FileState, entities::file},
    pagination::ChronologicalPageRequest,
    posts::PostState,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, EntityTrait,
    IntoActiveModel, JoinType, QueryFilter, QueryOrder, QuerySelect, Set, sea_query::LockType,
};
use std::collections::BTreeMap;
use uuid::Uuid;

pub(super) async fn insert_comment(
    transaction: &DatabaseTransaction,
    post_id: Uuid,
    author_id: Uuid,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Result<comment::Model, sea_orm::DbErr> {
    comment::ActiveModel {
        id: Set(Uuid::now_v7()),
        post_id: Set(post_id),
        author_id: Set(author_id),
        body: Set(body),
        deleted: Set(false),
        version: Set(1),
        created_at: Set(created_at),
    }
    .insert(transaction)
    .await
}

pub(super) async fn list_comments(
    connection: &impl ConnectionTrait,
    post_id: Uuid,
    page: &ChronologicalPageRequest,
) -> Result<Vec<comment::Model>, sea_orm::DbErr> {
    comment::Entity::find()
        .filter(comment::Column::PostId.eq(post_id))
        .filter(comment::Column::Deleted.eq(false))
        .filter(page.condition(comment::Column::CreatedAt, comment::Column::Id))
        .order_by_desc(comment::Column::CreatedAt)
        .order_by_desc(comment::Column::Id)
        .limit(page.limit + 1)
        .all(connection)
        .await
}

pub(super) async fn lock_comment(
    transaction: &DatabaseTransaction,
    post_id: Uuid,
    comment_id: Uuid,
) -> Result<Option<comment::Model>, sea_orm::DbErr> {
    comment::Entity::find_by_id(comment_id)
        .filter(comment::Column::PostId.eq(post_id))
        .filter(comment::Column::Deleted.eq(false))
        .lock(LockType::NoKeyUpdate)
        .one(transaction)
        .await
}

pub(super) async fn tombstone_comment(
    transaction: &DatabaseTransaction,
    comment: comment::Model,
    next_version: i64,
) -> Result<(), sea_orm::DbErr> {
    let mut updated = comment.into_active_model();
    updated.deleted = Set(true);
    updated.body = Set(String::new());
    updated.version = Set(next_version);
    updated.update(transaction).await?;
    Ok(())
}

pub(crate) fn published_condition() -> Condition {
    Condition::all().add(post::Column::State.eq(PostState::Published))
}

pub(super) async fn lock(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
    post_id: Uuid,
) -> Result<Option<post::Model>, sea_orm::DbErr> {
    post::Entity::find_by_id(post_id)
        .filter(post::Column::OrganizationId.eq(organization_id))
        .lock(LockType::NoKeyUpdate)
        .one(transaction)
        .await
}

pub(super) async fn attachments(
    connection: &impl ConnectionTrait,
    organization_id: Uuid,
    post_ids: Vec<Uuid>,
) -> Result<BTreeMap<Uuid, Vec<Uuid>>, sea_orm::DbErr> {
    let rows = post_attachment::Entity::find()
        .join(
            JoinType::InnerJoin,
            post_attachment::Entity::belongs_to(file::Entity)
                .from(post_attachment::Column::FileId)
                .to(file::Column::Id)
                .into(),
        )
        .filter(post_attachment::Column::PostId.is_in(post_ids))
        .filter(file::Column::OrganizationId.eq(organization_id))
        .filter(file::Column::State.eq(FileState::Ready))
        .order_by_asc(post_attachment::Column::FileId)
        .all(connection)
        .await?;
    let mut attachments = BTreeMap::<Uuid, Vec<Uuid>>::new();
    for row in rows {
        attachments
            .entry(row.post_id)
            .or_default()
            .push(row.file_id);
    }
    Ok(attachments)
}
