use crate::config::db_config::DbConfig;
use crate::dto::auth::internal::access_token::AccessTokenClaims;
use crate::dto::auth::internal::refresh_token::{JWTRefreshTokenResult, RefreshTokenClaims};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::de::DeserializeOwned;
use uuid::Uuid;

pub fn create_jwt_access_token(user_id: &Uuid) -> Result<String, jsonwebtoken::errors::Error> {
    let jwt_secret = &DbConfig::get().jwt_secret;
    let access_token_lifetime = DbConfig::get().auth_access_token_expire_time;
    let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());

    let now = Utc::now();
    let access_token_expires_at = now + Duration::minutes(access_token_lifetime);

    let claims = AccessTokenClaims {
        sub: *user_id,
        iat: now.timestamp(),
        exp: access_token_expires_at.timestamp(),
    };
    encode(&Header::default(), &claims, &encoding_key)
}

pub fn create_jwt_refresh_token(
    user_id: &Uuid,
) -> Result<JWTRefreshTokenResult, jsonwebtoken::errors::Error> {
    let jwt_secret = &DbConfig::get().jwt_secret;
    let refresh_token_lifetime = DbConfig::get().auth_refresh_token_expire_time;
    let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());

    let now = Utc::now();
    let refresh_token_expires_at = now + Duration::days(refresh_token_lifetime);

    let jti_value = Uuid::new_v4();

    let claims = RefreshTokenClaims {
        sub: *user_id,
        jti: jti_value,
        iat: now.timestamp(),
        exp: refresh_token_expires_at.timestamp(),
    };

    let token = encode(&Header::default(), &claims, &encoding_key)?;

    Ok(JWTRefreshTokenResult {
        token,
        jti: jti_value,
        expires_at: refresh_token_expires_at,
        issued_at: now,
    })
}

pub fn decode_token<T: DeserializeOwned>(
    token: &str,
) -> Result<TokenData<T>, jsonwebtoken::errors::Error> {
    let jwt_secret = &DbConfig::get().jwt_secret;
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::new(Algorithm::HS256);
    decode::<T>(token, &decoding_key, &validation)
}

pub fn decode_access_token(
    token: &str,
) -> Result<TokenData<AccessTokenClaims>, jsonwebtoken::errors::Error> {
    decode_token::<AccessTokenClaims>(token)
}

pub fn decode_refresh_token(
    token: &str,
) -> Result<TokenData<RefreshTokenClaims>, jsonwebtoken::errors::Error> {
    decode_token::<RefreshTokenClaims>(token)
}
