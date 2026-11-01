use super::fcm::{FcmClient, FcmSettings};
use crate::{ApplicationResult, identity::IdentityService};
use std::sync::Arc;

#[derive(Clone)]
pub struct NotificationService {
    pub(super) identity: IdentityService,
    pub(super) fcm: Option<Arc<FcmClient>>,
}
impl NotificationService {
    pub fn new(
        identity: IdentityService,
        settings: Option<FcmSettings>,
    ) -> ApplicationResult<Self> {
        Ok(Self {
            identity,
            fcm: settings.map(FcmClient::new).transpose()?.map(Arc::new),
        })
    }
}
