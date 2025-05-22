use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AccessTokenClaims {
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64, // Expiration time (Unix timestamp)
}
