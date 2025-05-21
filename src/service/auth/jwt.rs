use crate::config::db_config::DbConfig;
use crate::dto::auth_dto::{AccessTokenClaims, RefreshTokenClaims};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use uuid::Uuid;

pub fn create_access_token(
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

pub fn decode_access_token(
    token: &str,
) -> Result<TokenData<AccessTokenClaims>, jsonwebtoken::errors::Error> {
    let jwt_secret = &DbConfig::get().jwt_secret;
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

    let validation = Validation::new(Algorithm::HS256);

    decode::<AccessTokenClaims>(token, &decoding_key, &validation)
}

pub fn create_refresh_token_jwt(
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

pub fn decode_refresh_token(
    token: &str,
) -> Result<TokenData<RefreshTokenClaims>, jsonwebtoken::errors::Error> {
    let jwt_secret = &DbConfig::get().jwt_secret;
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

    let validation = Validation::new(Algorithm::HS256);

    decode::<RefreshTokenClaims>(token, &decoding_key, &validation)
}
