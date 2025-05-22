use crate::config::db_config::DbConfig;
use crate::payload::auth_payload::{AccessTokenClaims, RefreshTokenClaims};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::de::DeserializeOwned;
use uuid::Uuid;

pub fn create_jwt_access_token(
    user_id: &Uuid,
    iat: i64,
    exp: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let jwt_secret = &DbConfig::get().jwt_secret;
    let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());

    let claims = AccessTokenClaims {
        sub: *user_id,
        iat,
        exp,
    };
    encode(&Header::default(), &claims, &encoding_key)
}

pub fn create_jwt_refresh_token(
    user_id: &Uuid,
    jti: Uuid,
    iat: i64,
    exp: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let jwt_secret = &DbConfig::get().jwt_secret;
    let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());
    let claims = RefreshTokenClaims {
        sub: *user_id,
        jti,
        iat,
        exp,
    };
    encode(&Header::default(), &claims, &encoding_key)
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
