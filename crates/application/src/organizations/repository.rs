use super::entities::{membership, organization};
use crate::{organizations::MembershipRole, pagination::PageRequest};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    sea_query::{LockType, OnConflict, Query},
};
use uuid::Uuid;

pub(super) async fn count_members(
    connection: &impl ConnectionTrait,
    organization_id: Uuid,
) -> Result<u64, DbErr> {
    membership::Entity::find()
        .filter(membership::Column::OrganizationId.eq(organization_id))
        .count(connection)
        .await
}

pub(super) async fn account_can_join(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
) -> Result<bool, DbErr> {
    use crate::identity::entities::account;
    Ok(account::Entity::find_by_id(account_id)
        .filter(account::Column::Active.eq(true))
        .filter(account::Column::Verified.eq(true))
        .one(connection)
        .await?
        .is_some())
}

pub(super) async fn insert_member(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
    account_id: Uuid,
) -> Result<(), DbErr> {
    membership::Entity::insert(membership::ActiveModel {
        id: Set(Uuid::now_v7()),
        organization_id: Set(organization_id),
        account_id: Set(account_id),
        role: Set(MembershipRole::Member),
    })
    .on_conflict(
        OnConflict::columns([
            membership::Column::OrganizationId,
            membership::Column::AccountId,
        ])
        .do_nothing()
        .to_owned(),
    )
    .exec_without_returning(transaction)
    .await?;
    Ok(())
}

pub(super) async fn delete_member(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
    account_id: Uuid,
) -> Result<u64, DbErr> {
    Ok(membership::Entity::delete_many()
        .filter(membership::Column::OrganizationId.eq(organization_id))
        .filter(membership::Column::AccountId.eq(account_id))
        .exec(transaction)
        .await?
        .rows_affected)
}

pub async fn insert_organization(
    transaction: &DatabaseTransaction,
    name: String,
    owner_id: Uuid,
) -> Result<organization::Model, DbErr> {
    let organization = organization::ActiveModel {
        id: Set(Uuid::now_v7()),
        name: Set(name),
        owner_id: Set(owner_id),
        ..Default::default()
    }
    .insert(transaction)
    .await?;
    membership::ActiveModel {
        id: Set(Uuid::now_v7()),
        organization_id: Set(organization.id),
        account_id: Set(owner_id),
        role: Set(MembershipRole::Owner),
    }
    .insert(transaction)
    .await?;
    Ok(organization)
}

pub async fn lock_organization(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
) -> Result<Option<organization::Model>, DbErr> {
    // Serialize organization changes without blocking child foreign-key checks.
    organization::Entity::find_by_id(organization_id)
        .lock(LockType::NoKeyUpdate)
        .one(transaction)
        .await
}

pub async fn find_organization(
    connection: &impl ConnectionTrait,
    organization_id: Uuid,
) -> Result<Option<organization::Model>, DbErr> {
    organization::Entity::find_by_id(organization_id)
        .one(connection)
        .await
}

pub async fn is_member(
    connection: &impl ConnectionTrait,
    organization_id: Uuid,
    account_id: Uuid,
) -> Result<bool, DbErr> {
    Ok(membership::Entity::find()
        .filter(membership::Column::OrganizationId.eq(organization_id))
        .filter(membership::Column::AccountId.eq(account_id))
        .one(connection)
        .await?
        .is_some())
}

pub async fn list_for_member(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
    page: PageRequest,
) -> Result<Vec<organization::Model>, DbErr> {
    let memberships = Query::select()
        .column(membership::Column::OrganizationId)
        .from(membership::Entity)
        .and_where(membership::Column::AccountId.eq(account_id))
        .to_owned();
    let mut query =
        organization::Entity::find().filter(organization::Column::Id.in_subquery(memberships));
    if let Some(after_id) = page.after_id {
        query = query.filter(organization::Column::Id.gt(after_id));
    }
    query
        .order_by_asc(organization::Column::Id)
        .limit(page.limit + 1)
        .all(connection)
        .await
}

pub async fn rename_organization(
    transaction: &DatabaseTransaction,
    organization: organization::Model,
    name: String,
) -> Result<organization::Model, DbErr> {
    let next_version = organization.version + 1;
    let mut organization = organization.into_active_model();
    organization.name = Set(name);
    organization.version = Set(next_version);
    organization.update(transaction).await
}
