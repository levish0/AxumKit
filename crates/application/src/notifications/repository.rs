use super::{
    NotificationPreferences,
    entities::{notification, notification_preference},
};
use crate::{
    organizations::entities::membership,
    posts::entities::{comment, post},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter, QuerySelect, Select,
};
use uuid::Uuid;

pub(super) fn visible(recipient_id: Uuid) -> Select<notification::Entity> {
    notification::Entity::find()
        .join(
            JoinType::InnerJoin,
            notification::Entity::belongs_to(post::Entity)
                .from(notification::Column::PostId)
                .to(post::Column::Id)
                .into(),
        )
        .join(
            JoinType::InnerJoin,
            notification::Entity::belongs_to(comment::Entity)
                .from(notification::Column::CommentId)
                .to(comment::Column::Id)
                .into(),
        )
        .join(
            JoinType::InnerJoin,
            notification::Entity::belongs_to(membership::Entity)
                .from(notification::Column::OrganizationId)
                .to(membership::Column::OrganizationId)
                .into(),
        )
        .filter(notification::Column::RecipientId.eq(recipient_id))
        .filter(membership::Column::AccountId.eq(recipient_id))
        .filter(crate::posts::published_condition())
        .filter(comment::Column::Deleted.eq(false))
}

pub(super) async fn preferences(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
) -> Result<NotificationPreferences, sea_orm::DbErr> {
    Ok(notification_preference::Entity::find()
        .filter(notification_preference::Column::AccountId.eq(account_id))
        .one(connection)
        .await?
        .map(|row| NotificationPreferences {
            post_comments: row.post_comments,
            push_enabled: row.push_enabled,
        })
        .unwrap_or_default())
}
