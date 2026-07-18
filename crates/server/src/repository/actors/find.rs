use entity::actors::{Column, Entity as ActorEntity, Model as ActorModel};
use entity::common::ActorKind;
use errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

pub async fn repository_find_actor_by_user_id<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<Option<ActorModel>, Errors>
where
    C: ConnectionTrait,
{
    ActorEntity::find()
        .filter(Column::UserId.eq(user_id))
        .one(conn)
        .await
        .map_err(Into::into)
}

pub async fn repository_find_actor_by_id<C>(
    conn: &C,
    actor_id: Uuid,
) -> Result<Option<ActorModel>, Errors>
where
    C: ConnectionTrait,
{
    ActorEntity::find_by_id(actor_id)
        .one(conn)
        .await
        .map_err(Into::into)
}

pub async fn repository_find_actor_by_anonymous_ip<C>(
    conn: &C,
    ip: IpNetwork,
) -> Result<Option<ActorModel>, Errors>
where
    C: ConnectionTrait,
{
    ActorEntity::find()
        .filter(Column::Kind.eq(ActorKind::Anonymous))
        .filter(Column::Ip.eq(ip))
        .one(conn)
        .await
        .map_err(Into::into)
}

pub async fn repository_find_actors_by_ids<C>(
    conn: &C,
    actor_ids: &[Uuid],
) -> Result<Vec<ActorModel>, Errors>
where
    C: ConnectionTrait,
{
    if actor_ids.is_empty() {
        return Ok(Vec::new());
    }

    ActorEntity::find()
        .filter(Column::Id.is_in(actor_ids.to_vec()))
        .all(conn)
        .await
        .map_err(Into::into)
}
