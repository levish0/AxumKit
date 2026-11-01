use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct OrganizationOutput {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub version: i64,
}
impl From<application::organizations::OrganizationSummary> for OrganizationOutput {
    fn from(organization: application::organizations::OrganizationSummary) -> Self {
        Self {
            id: organization.id,
            name: organization.name,
            owner_id: organization.owner_id,
            version: organization.version,
        }
    }
}
