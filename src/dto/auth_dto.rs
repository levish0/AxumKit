use axum::Json;
use axum::http::HeaderValue;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Response};
use cookie::Cookie;
use cookie::time::Duration;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct AuthLoginRequest {
    #[validate(length(
        min = 3,
        max = 20,
        message = "Handle must be between 3 and 20 characters."
    ))]
    pub handle: String,
    #[validate(length(
        min = 6,
        max = 20,
        message = "Password must be between 6 and 20 characters."
    ))]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthLoginResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct AuthLoginAccessTokenResponse {
    pub access_token: String,
    #[serde(skip_serializing)]
    pub refresh_token: String,
}

impl IntoResponse for AuthLoginAccessTokenResponse {
    fn into_response(self) -> Response {
        let mut response = Json(AuthLoginAccessTokenResponse {
            access_token: self.access_token.clone(),
            refresh_token: String::new(),
        })
        .into_response();

        let cookie = Cookie::build(("refresh_token", self.refresh_token))
            .http_only(true)
            .path("/")
            .max_age(Duration::days(14))
            .build();

        response.headers_mut().insert(
            SET_COOKIE,
            HeaderValue::from_str(&cookie.to_string()).unwrap(),
        );

        response
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AccessTokenClaims {
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64, // Expiration time (Unix timestamp)
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RefreshTokenClaims {
    pub sub: Uuid, // User ID (e.g., Uuid as string)
    pub jti: Uuid, // JWT ID, a unique identifier for this specific refresh token
    pub iat: i64,  // Issued At (Unix timestamp)
    pub exp: i64,  // Expiration Time (Unix timestamp)
}
