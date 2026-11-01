use super::entities::profile;
use crate::{identity::entities::account, organizations::entities::membership};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
    Select,
};
use uuid::Uuid;

pub(super) fn visible_in_organization(organization_id: Uuid) -> Select<profile::Entity> {
    profile::Entity::find()
        .join(
            JoinType::InnerJoin,
            profile::Entity::belongs_to(account::Entity)
                .from(profile::Column::AccountId)
                .to(account::Column::Id)
                .into(),
        )
        .join(
            JoinType::InnerJoin,
            profile::Entity::belongs_to(membership::Entity)
                .from(profile::Column::AccountId)
                .to(membership::Column::AccountId)
                .into(),
        )
        .filter(account::Column::Active.eq(true))
        .filter(account::Column::Verified.eq(true))
        .filter(membership::Column::OrganizationId.eq(organization_id))
}

pub(super) async fn for_account(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
) -> Result<Option<profile::Model>, sea_orm::DbErr> {
    profile::Entity::find()
        .filter(profile::Column::AccountId.eq(account_id))
        .one(connection)
        .await
}

pub(super) async fn organization_ids(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
) -> Result<Vec<Uuid>, sea_orm::DbErr> {
    membership::Entity::find()
        .select_only()
        .column(membership::Column::OrganizationId)
        .filter(membership::Column::AccountId.eq(account_id))
        .order_by_asc(membership::Column::OrganizationId)
        .into_tuple()
        .all(connection)
        .await
}

pub(super) async fn active(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
) -> Result<bool, sea_orm::DbErr> {
    Ok(account::Entity::find_by_id(account_id)
        .filter(account::Column::Active.eq(true))
        .filter(account::Column::Verified.eq(true))
        .one(connection)
        .await?
        .is_some())
}
