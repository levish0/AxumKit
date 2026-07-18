use super::{
    repository_create_anonymous_actor, repository_create_user_actor,
    repository_find_actor_by_anonymous_ip, repository_find_actor_by_user_id,
};
use entity::actors::Model as ActorModel;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;
use sea_orm::prelude::IpNetwork;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct ActorIdentity {
    pub user_id: Option<Uuid>,
    pub ip: Option<IpNetwork>,
}

pub async fn repository_find_or_create_actor_for_identity<C>(
    conn: &C,
    identity: ActorIdentity,
) -> Result<ActorModel, Errors>
where
    C: ConnectionTrait,
{
    match (identity.user_id, identity.ip) {
        (Some(user_id), _) => repository_find_or_create_user_actor(conn, user_id).await,
        (None, Some(ip)) => repository_find_or_create_anonymous_actor(conn, ip).await,
        (None, None) => Err(Errors::BadRequestError(
            "actor identity requires user_id or ip".to_string(),
        )),
    }
}

pub async fn repository_find_or_create_user_actor<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<ActorModel, Errors>
where
    C: ConnectionTrait,
{
    if let Some(actor) = repository_find_actor_by_user_id(conn, user_id).await? {
        return Ok(actor);
    }

    match repository_create_user_actor(conn, user_id).await {
        Ok(actor) => Ok(actor),
        Err(Errors::DatabaseError(msg)) if msg.contains("duplicate") || msg.contains("unique") => {
            repository_find_actor_by_user_id(conn, user_id)
                .await?
                .ok_or(Errors::DatabaseError(msg))
        }
        Err(e) => Err(e),
    }
}

pub async fn repository_find_or_create_anonymous_actor<C>(
    conn: &C,
    ip: IpNetwork,
) -> Result<ActorModel, Errors>
where
    C: ConnectionTrait,
{
    if let Some(actor) = repository_find_actor_by_anonymous_ip(conn, ip).await? {
        return Ok(actor);
    }

    match repository_create_anonymous_actor(conn, ip).await {
        Ok(actor) => Ok(actor),
        Err(Errors::DatabaseError(msg)) if msg.contains("duplicate") || msg.contains("unique") => {
            repository_find_actor_by_anonymous_ip(conn, ip)
                .await?
                .ok_or(Errors::DatabaseError(msg))
        }
        Err(e) => Err(e),
    }
}
