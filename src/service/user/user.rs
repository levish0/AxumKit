use crate::dto::user_dto::{CreateUserRequest, UserInfoResponse};
use crate::entity::user::{
    ActiveModel as UserActiveModel, Entity as UserEntity
};
use crate::service::error::errors::Errors;
use anyhow::Result;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use tracing::error;

pub async fn service_create_user(
    conn: &DatabaseConnection,
    payload: CreateUserRequest,
) -> Result<(), Errors> {
    let new_user = UserActiveModel {
        id: Default::default(),
        username: Set(payload.username),
        email: Set(payload.email),
        password: Set(payload.password),
    };

    match new_user.insert(conn).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to insert user: {:?}", e);
            Err(Errors::DatabaseError(e.to_string()))
        }
    }
}

pub async fn service_get_user(
    conn: &DatabaseConnection,
    id: i32,
) -> Result<UserInfoResponse, Errors> {
    let user = UserEntity::find_by_id(id)
        .one(conn)
        .await
        .map_err(|e| Errors::DatabaseError(e.to_string()))?;

    match user {
        Some(user) => Ok(UserInfoResponse {
            username: user.username,
            email: user.email,
        }),
        None => {
            error!("User not found with id: {}", id);
            Err(Errors::UserNotFound)
        }
    }
}
