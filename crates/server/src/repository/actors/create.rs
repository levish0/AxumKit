use entity::actors::{ActiveModel as ActorActiveModel, Model as ActorModel};
use entity::common::ActorKind;
use errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

pub async fn repository_create_user_actor<C>(conn: &C, user_id: Uuid) -> Result<ActorModel, Errors>
where
    C: ConnectionTrait,
{
    ActorActiveModel {
        id: Default::default(),
        kind: Set(ActorKind::User),
        user_id: Set(Some(user_id)),
        ip: Set(None),
        created_at: Default::default(),
    }
    .insert(conn)
    .await
    .map_err(Into::into)
}

pub async fn repository_create_anonymous_actor<C>(
    conn: &C,
    ip: IpNetwork,
) -> Result<ActorModel, Errors>
where
    C: ConnectionTrait,
{
    ActorActiveModel {
        id: Default::default(),
        kind: Set(ActorKind::Anonymous),
        user_id: Set(None),
        ip: Set(Some(ip)),
        created_at: Default::default(),
    }
    .insert(conn)
    .await
    .map_err(Into::into)
}

pub async fn repository_create_system_actor<C>(conn: &C) -> Result<ActorModel, Errors>
where
    C: ConnectionTrait,
{
    ActorActiveModel {
        id: Default::default(),
        kind: Set(ActorKind::System),
        user_id: Set(None),
        ip: Set(None),
        created_at: Default::default(),
    }
    .insert(conn)
    .await
    .map_err(Into::into)
}
