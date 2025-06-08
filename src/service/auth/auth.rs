use crate::dto::auth::internal::refresh_token::RefreshTokenClaims;
use crate::dto::auth::request::login::AuthLoginRequest;
use crate::dto::auth::response::jwt::AuthJWTResponse;
use crate::entity::user_refresh_tokens::{
    ActiveModel as RefreshTokenActiveModel, Column as RefreshTokenColumn,
    Entity as RefreshTokenEntity,
};
use crate::entity::users::{Column as UserColumn, Entity as UserEntity};
use crate::service::auth::jwt::{create_jwt_access_token, create_jwt_refresh_token};
use crate::service::error::errors::Errors;
use crate::service::user::crypto::verify_password;
use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tracing::error;

pub async fn service_login(
    conn: &DatabaseConnection,
    payload: AuthLoginRequest,
) -> Result<AuthJWTResponse, Errors> {
    let user = UserEntity::find()
        .filter(UserColumn::Handle.eq(&payload.handle))
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

    let access_token = create_jwt_access_token(&user.id).map_err(|e| {
        error!("Failed to create access token: {:?}", e);
        Errors::TokenCreationError(e.to_string())
    })?;
    let refresh_token = create_jwt_refresh_token(&user.id).map_err(|e| {
        error!("Failed to create refresh token: {:?}", e);
        Errors::TokenCreationError(e.to_string())
    })?;

    let refresh_model = RefreshTokenActiveModel {
        id: Set(refresh_token.jti),
        user_id: Set(user.id),
        ip_address: Default::default(),
        user_agent: Default::default(),
        refresh_token: Set(refresh_token.token.clone()),
        expires_at: Set(refresh_token.expires_at),
        created_at: Set(refresh_token.issued_at),
        revoked_at: Default::default(),
    };

    match refresh_model.insert(conn).await {
        Ok(_) => Ok(AuthJWTResponse {
            access_token,
            cookie_refresh_token: refresh_token.token,
        }),
        Err(e) => {
            error!("Failed to login user: {:?}", e);
            Err(Errors::DatabaseError(e.to_string()))
        }
    }
}

pub async fn service_refresh(
    conn: &DatabaseConnection,
    refresh_token: String,
    refresh_token_claims: RefreshTokenClaims,
) -> Result<AuthJWTResponse, Errors> {
    // Validate that the token isn't expired
    let now = Utc::now().timestamp();
    if refresh_token_claims.exp < now {
        error!(
            "Refresh token has expired: token_exp={}, now={}",
            refresh_token_claims.exp, now
        );
        return Err(Errors::UserTokenExpired);
    }
    let stored_token = RefreshTokenEntity::find()
        .filter(RefreshTokenColumn::Id.eq(refresh_token_claims.jti))
        .filter(RefreshTokenColumn::RefreshToken.eq(refresh_token))
        .filter(RefreshTokenColumn::RevokedAt.is_null()) // revoked 되지 않은 토큰만
        .one(conn)
        .await
        .map_err(|e| {
            error!("Database error while fetching refresh token: {:?}", e);
            Errors::DatabaseError(e.to_string())
        })?;

    let stored_token = match stored_token {
        Some(token) => token,
        None => {
            error!("Refresh token not found or revoked");
            return Err(Errors::UserInvalidToken);
        }
    };

    let user = UserEntity::find()
        .filter(UserColumn::Id.eq(refresh_token_claims.sub))
        .one(conn)
        .await
        .map_err(|e| {
            error!("Database error while fetching user: {:?}", e);
            Errors::DatabaseError(e.to_string())
        })?;

    let user = match user {
        Some(u) => u,
        None => {
            error!("User not found for refresh token");
            return Err(Errors::UserNotFound);
        }
    };

    let mut revoke_model: RefreshTokenActiveModel = stored_token.into();
    revoke_model.revoked_at = Set(Some(Utc::now()));

    revoke_model.update(conn).await.map_err(|e| {
        error!("Failed to revoke old refresh token: {:?}", e);
        Errors::DatabaseError(e.to_string())
    })?;

    let new_access_token = create_jwt_access_token(&user.id).map_err(|e| {
        error!("Failed to create new access token: {:?}", e);
        Errors::TokenCreationError(e.to_string())
    })?;

    let new_refresh_token = create_jwt_refresh_token(&user.id).map_err(|e| {
        error!("Failed to create new refresh token: {:?}", e);
        Errors::TokenCreationError(e.to_string())
    })?;

    let new_refresh_model = RefreshTokenActiveModel {
        id: Set(new_refresh_token.jti),
        user_id: Set(user.id),
        ip_address: Default::default(),
        user_agent: Default::default(),
        refresh_token: Set(new_refresh_token.token.clone()),
        expires_at: Set(new_refresh_token.expires_at),
        created_at: Set(new_refresh_token.issued_at),
        revoked_at: Default::default(),
    };

    match new_refresh_model.insert(conn).await {
        Ok(_) => Ok(AuthJWTResponse {
            access_token: new_access_token,
            cookie_refresh_token: new_refresh_token.token,
        }),
        Err(e) => {
            error!("Failed to login user: {:?}", e);
            Err(Errors::DatabaseError(e.to_string()))
        }
    }
}
