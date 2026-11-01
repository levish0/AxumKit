mod scope;
pub use scope::ApiScope;
mod admission;
pub mod entities;
mod management;
mod repository;
mod types;
pub mod validation;

use crate::{identity::IdentityService, organizations::OrganizationService};

pub use types::{
    ApiAdmission, ApiClientSummary, ApiKeySummary, ExternalOrganization, IssuedApiKey, QuotaStatus,
};

#[derive(Clone)]
pub struct ApiAccessService {
    identity: IdentityService,
    organizations: OrganizationService,
}
impl ApiAccessService {
    pub fn new(identity: IdentityService, organizations: OrganizationService) -> Self {
        Self {
            identity,
            organizations,
        }
    }
}
