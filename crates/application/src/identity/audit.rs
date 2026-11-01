use super::{IdentityService, entities::audit_event};
use crate::ApplicationResult;
use jobs::TraceContext;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};
use uuid::Uuid;

impl IdentityService {
    pub(crate) async fn record_resource_security_event(
        &self,
        transaction: &DatabaseTransaction,
        actor_id: Option<Uuid>,
        event: &'static str,
        resource_type: &'static str,
        resource_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        audit_event::ActiveModel {
            id: Set(Uuid::now_v7()),
            account_id: Set(actor_id),
            event: Set(event.into()),
            resource_type: Set(Some(resource_type.into())),
            resource_id: Set(Some(resource_id)),
            request_id: Set(trace.request_id),
            ..Default::default()
        }
        .insert(transaction)
        .await?;
        Ok(())
    }
}
