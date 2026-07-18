use crate::repository::actors::repository_find_actors_by_ids;
use crate::repository::user::find_by_ids::repository_find_users_by_ids;
use crate::service::user::mapper::mask_user_identity;
use dto::actor::ActorResponse;
use entity::actors::Model as ActorModel;
use entity::common::ActorKind;
use entity::users::Model as UserModel;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;
use std::collections::HashMap;
use uuid::Uuid;

pub fn actor_response(actor: &ActorModel, users: &HashMap<Uuid, UserModel>) -> ActorResponse {
    match actor.kind {
        ActorKind::User => {
            // Under soft delete a user-kind actor's user row is always preserved, so it resolves
            // normally. Masking is unified through mask_user_identity.
            match actor.user_id.and_then(|user_id| users.get(&user_id)) {
                Some(user) => {
                    let identity = mask_user_identity(user);
                    ActorResponse {
                        id: actor.id,
                        kind: ActorKind::User,
                        user_id: Some(identity.id),
                        handle: Some(identity.handle),
                        display_name: Some(identity.display_name),
                        profile_image: identity.profile_image,
                        ip: None,
                        deactivated: identity.deactivated,
                    }
                }
                // A missing user is a data-integrity bug; treat it as deactivated and mask.
                None => ActorResponse {
                    id: actor.id,
                    kind: ActorKind::User,
                    user_id: None,
                    handle: None,
                    display_name: None,
                    profile_image: None,
                    ip: None,
                    deactivated: true,
                },
            }
        }
        ActorKind::Anonymous => ActorResponse {
            id: actor.id,
            kind: actor.kind,
            user_id: None,
            handle: None,
            display_name: None,
            profile_image: None,
            ip: actor.ip.map(|ip| ip.to_string()),
            deactivated: false,
        },
        ActorKind::System => ActorResponse {
            id: actor.id,
            kind: actor.kind,
            user_id: None,
            handle: None,
            display_name: None,
            profile_image: None,
            ip: None,
            deactivated: false,
        },
    }
}

pub async fn actor_response_map<C>(
    conn: &C,
    actor_ids: &[Uuid],
) -> Result<HashMap<Uuid, ActorResponse>, Errors>
where
    C: ConnectionTrait,
{
    let actors = repository_find_actors_by_ids(conn, actor_ids).await?;
    let user_ids: Vec<Uuid> = actors.iter().filter_map(|actor| actor.user_id).collect();
    let users: HashMap<Uuid, UserModel> = repository_find_users_by_ids(conn, &user_ids)
        .await?
        .into_iter()
        .map(|user| (user.id, user))
        .collect();

    Ok(actors
        .iter()
        .map(|actor| (actor.id, actor_response(actor, &users)))
        .collect())
}

pub async fn actor_response_by_id<C>(
    conn: &C,
    actor_id: Uuid,
) -> Result<Option<ActorResponse>, Errors>
where
    C: ConnectionTrait,
{
    Ok(actor_response_map(conn, &[actor_id])
        .await?
        .remove(&actor_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::prelude::IpNetwork;

    // Active users keep images off so the masking tests never reach `build_r2_public_url`
    // (which requires a global ServerConfig).
    fn make_user(id: Uuid, deleted: bool, with_image: bool) -> UserModel {
        let now = Utc::now();
        UserModel {
            id,
            display_name: "Alice".to_string(),
            handle: "alice".to_string(),
            bio: Some("hi".to_string()),
            email: "alice@example.com".to_string(),
            password: None,
            profile_image: with_image.then(|| "avatar-key".to_string()),
            banner_image: None,
            totp_secret: None,
            totp_enabled_at: None,
            totp_backup_codes: None,
            created_at: now,
            deleted_at: deleted.then_some(now),
        }
    }

    fn make_actor(kind: ActorKind, user_id: Option<Uuid>, ip: Option<IpNetwork>) -> ActorModel {
        ActorModel {
            id: Uuid::new_v4(),
            kind,
            user_id,
            ip,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn user_actor_active_is_not_deactivated() {
        let uid = Uuid::new_v4();
        let users = HashMap::from([(uid, make_user(uid, false, false))]);
        let resp = actor_response(&make_actor(ActorKind::User, Some(uid), None), &users);
        assert_eq!(resp.kind, ActorKind::User);
        assert!(!resp.deactivated);
        assert_eq!(resp.user_id, Some(uid));
        assert_eq!(resp.handle.as_deref(), Some("alice"));
    }

    #[test]
    fn user_actor_deactivated_masks_image_keeps_handle() {
        let uid = Uuid::new_v4();
        let users = HashMap::from([(uid, make_user(uid, true, true))]);
        let resp = actor_response(&make_actor(ActorKind::User, Some(uid), None), &users);
        assert!(resp.deactivated);
        assert_eq!(resp.handle.as_deref(), Some("alice"));
        assert_eq!(resp.display_name.as_deref(), Some("Alice"));
        assert_eq!(resp.profile_image, None);
    }

    #[test]
    fn user_actor_with_missing_user_is_masked() {
        let users: HashMap<Uuid, UserModel> = HashMap::new();
        let resp = actor_response(
            &make_actor(ActorKind::User, Some(Uuid::new_v4()), None),
            &users,
        );
        assert!(resp.deactivated);
        assert_eq!(resp.user_id, None);
        assert_eq!(resp.handle, None);
    }

    #[test]
    fn anonymous_actor_exposes_ip() {
        let ip: IpNetwork = "203.0.113.7".parse().unwrap();
        let resp = actor_response(
            &make_actor(ActorKind::Anonymous, None, Some(ip)),
            &HashMap::new(),
        );
        assert_eq!(resp.kind, ActorKind::Anonymous);
        assert!(!resp.deactivated);
        assert!(resp.ip.unwrap().contains("203.0.113.7"));
        assert_eq!(resp.handle, None);
    }

    #[test]
    fn system_actor_is_not_deactivated() {
        let resp = actor_response(&make_actor(ActorKind::System, None, None), &HashMap::new());
        assert_eq!(resp.kind, ActorKind::System);
        assert!(!resp.deactivated);
        assert_eq!(resp.handle, None);
    }
}
