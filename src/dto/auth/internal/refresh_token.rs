use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RefreshTokenContext {
    pub token: String,
    pub claims: RefreshTokenClaims,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshTokenClaims {
    pub sub: Uuid, // User ID (e.g., Uuid as string)
    pub jti: Uuid, // JWT ID, a unique identifier for this specific refresh token
    pub iat: i64,  // Issued At (Unix timestamp)
    pub exp: i64,  // Expiration Time (Unix timestamp)
}
pub struct JWTRefreshTokenResult {
    pub token: String,
    pub jti: Uuid,
    pub expires_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
}
