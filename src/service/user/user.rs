use crate::dto::user_dto::{CreateUserRequest, UserInfoResponse};
use crate::entity::users::{ActiveModel as UserActiveModel, Column, Entity as UserEntity};
use crate::service::error::errors::Errors;
use crate::service::user::crypto::hash_password;
use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tracing::error;
use uuid::Uuid;

pub async fn service_create_user(
    conn: &DatabaseConnection,
    payload: CreateUserRequest,
) -> Result<(), Errors> {
    let hashed_password = hash_password(&payload.password)?;

    let new_user = UserActiveModel {
        id: Default::default(),
        name: Set(payload.name),
        handle: Set(payload.handle),
        email: Set(payload.email),
        password: Set(hashed_password),
    };

    match new_user.insert(conn).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to insert user: {:?}", e);
            Err(Errors::DatabaseError(e.to_string()))
        }
    }
}

pub async fn service_get_user_by_uuid(
    conn: &DatabaseConnection,
    user_uuid: &Uuid,
) -> Result<UserInfoResponse, Errors> {
    let user = UserEntity::find_by_id(*user_uuid)
        .one(conn)
        .await
        .map_err(|e| Errors::DatabaseError(e.to_string()))?;

    match user {
        Some(user) => Ok(UserInfoResponse {
            name: user.name,
            handle: user.handle,
            email: user.email,
        }),
        None => {
            error!("User not found with id: {}", user_uuid);
            Err(Errors::UserNotFound)
        }
    }
}

pub async fn service_get_user_by_handle(
    conn: &DatabaseConnection,
    handle: &str,
) -> Result<UserInfoResponse, Errors> {
    let user = UserEntity::find()
        .filter(Column::Handle.eq(handle))
        .one(conn)
        .await
        .map_err(|e| Errors::DatabaseError(e.to_string()))?;

    match user {
        Some(user) => Ok(UserInfoResponse {
            name: user.name,
            handle: user.handle,
            email: user.email,
        }),
        None => {
            error!("User not found with handle: {}", handle);
            Err(Errors::UserNotFound)
        }
    }
}
