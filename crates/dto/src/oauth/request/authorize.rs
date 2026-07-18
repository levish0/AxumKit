use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(
    description = "Controls whether the generated OAuth authorization URL is used for sign-in or for linking an already authenticated account."
)]
#[serde(rename_all = "lowercase")]
/// Request enum for o auth authorize flow.
pub enum OAuthAuthorizeFlow {
    Login,
    Link,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
#[schema(description = "Query parameters for creating an OAuth authorization URL.")]
/// Request payload for o auth authorize query.
pub struct OAuthAuthorizeQuery {
    #[serde(default)]
    pub flow: Option<OAuthAuthorizeFlow>,
}
