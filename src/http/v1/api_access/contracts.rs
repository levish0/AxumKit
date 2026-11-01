use crate::http::validation::api_access_name;
use application::api_access::{ApiClientSummary, ApiKeySummary, QuotaStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApiClientInput {
    #[validate(custom(function = "api_access_name"))]
    #[schema(min_length = 1, max_length = 100)]
    pub name: String,
    /// Allowed values: organizations:read, posts:read. Empty scopes deny all external operations.
    #[validate(custom(function = "validate_scopes"))]
    pub scopes: Vec<ApiScope>,
}

#[derive(Serialize, ToSchema)]
pub struct QuotaOutput {
    pub minute_limit: i64,
    pub minute_used: i64,
    pub minute_resets_at: DateTime<Utc>,
    pub month_limit: i64,
    pub month_used: i64,
    pub month_resets_at: DateTime<Utc>,
}
impl From<QuotaStatus> for QuotaOutput {
    fn from(value: QuotaStatus) -> Self {
        Self {
            minute_limit: value.minute_limit,
            minute_used: value.minute_used,
            minute_resets_at: value.minute_resets_at,
            month_limit: value.month_limit,
            month_used: value.month_used,
            month_resets_at: value.month_resets_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiClientOutput {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub scopes: Vec<ApiScope>,
    pub active: bool,
    pub quota: QuotaOutput,
}
impl From<ApiClientSummary> for ApiClientOutput {
    fn from(value: ApiClientSummary) -> Self {
        Self {
            id: value.id,
            organization_id: value.organization_id,
            name: value.name,
            scopes: value.scopes.into_iter().map(Into::into).collect(),
            active: value.active,
            quota: value.quota.into(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiKeyOutput {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
impl From<ApiKeySummary> for ApiKeyOutput {
    fn from(value: ApiKeySummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            created_at: value.created_at,
            expires_at: value.expires_at,
            revoked_at: value.revoked_at,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiScope {
    #[serde(rename = "organizations:read")]
    OrganizationsRead,
    #[serde(rename = "posts:read")]
    PostsRead,
}
impl From<application::api_access::ApiScope> for ApiScope {
    fn from(value: application::api_access::ApiScope) -> Self {
        match value {
            application::api_access::ApiScope::OrganizationsRead => Self::OrganizationsRead,
            application::api_access::ApiScope::PostsRead => Self::PostsRead,
        }
    }
}
impl From<ApiScope> for application::api_access::ApiScope {
    fn from(value: ApiScope) -> Self {
        match value {
            ApiScope::OrganizationsRead => Self::OrganizationsRead,
            ApiScope::PostsRead => Self::PostsRead,
        }
    }
}

fn validate_scopes(values: &[ApiScope]) -> Result<(), validator::ValidationError> {
    let scopes: Vec<application::api_access::ApiScope> =
        values.iter().copied().map(Into::into).collect();
    application::api_access::validation::scopes(&scopes)
        .map_err(|_| validator::ValidationError::new("api_scopes"))
}
