use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
/// Compact public user identity for nested response fields.
pub struct UserBriefResponse {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image: Option<String>,
    /// True if the user is deactivated (soft-deleted); handle/display_name stay exposed while the profile is masked.
    pub deactivated: bool,
}
