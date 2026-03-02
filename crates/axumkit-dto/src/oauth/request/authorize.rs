use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OAuthAuthorizeFlow {
    Login,
    Link,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct OAuthAuthorizeQuery {
    #[serde(default)]
    pub flow: Option<OAuthAuthorizeFlow>,
}
