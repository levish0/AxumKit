use crate::entity::user_refresh_tokens::ActiveModel as RefreshTokenActiveModel;
use crate::entity::users::{Column, Entity as UserEntity};
use crate::payload::auth_payload::{AuthLoginRequest, AuthLoginResponse};
use crate::service::auth::jwt::{create_jwt_access_token, create_jwt_refresh_token};
use crate::service::error::errors::Errors;
use crate::service::user::crypto::verify_password;
use anyhow::Result;
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tracing::error;
use uuid::Uuid;

pub async fn service_login(
    conn: &DatabaseConnection,
    payload: AuthLoginRequest,
) -> Result<AuthLoginResponse, Errors> {
    let user = UserEntity::find()
        .filter(Column::Handle.eq(&payload.handle))
        .one(conn)
        .await
        .map_err(|e| Errors::DatabaseError(e.to_string()))?;

    let user = match user {
        Some(u) => u,
        None => {
            error!("User not found with handle: {}", payload.handle);
            return Err(Errors::UserNotFound);
        }
    };

    verify_password(&payload.password, &user.password)?;

    let now = Utc::now();
    let access_token_expires_at = now + Duration::hours(1);
    let refresh_token_expires_at = now + Duration::days(14);
    let access_token = create_jwt_access_token(
        &user.id,
        now.timestamp(),
        access_token_expires_at.timestamp(),
    )
    .unwrap();
    let jti_value = Uuid::new_v4();
    let refresh_token = create_jwt_refresh_token(
        &user.id,
        jti_value,
        now.timestamp(),
        refresh_token_expires_at.timestamp(),
    )
    .unwrap();

    let refresh_model = RefreshTokenActiveModel {
        id: Set(jti_value),
        user_id: Set(user.id),
        refresh_token: Set(refresh_token.clone()),
        expires_at: Set(refresh_token_expires_at),
        created_at: Set(now),
        revoked_at: Default::default(),
    };

    match refresh_model.insert(conn).await {
        Ok(_) => Ok(AuthLoginResponse {
            access_token,
            refresh_token,
        }),
        Err(e) => {
            error!("Failed to login user: {:?}", e);
            Err(Errors::DatabaseError(e.to_string()))
        }
    }
}
