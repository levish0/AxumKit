use super::{PROFILE_SEARCH_JOB, repository};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub(super) struct ProfileJob {
    pub account_id: Uuid,
}

pub(crate) async fn enqueue_search(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: uuid::Uuid,
    trace: jobs::TraceContext,
) -> crate::ApplicationResult<()> {
    jobs::enqueue(
        transaction,
        uuid::Uuid::now_v7(),
        PROFILE_SEARCH_JOB,
        &ProfileJob { account_id },
        trace,
    )
    .await?;
    Ok(())
}

pub(crate) async fn membership_changed(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: uuid::Uuid,
) -> crate::ApplicationResult<()> {
    if repository::for_account(transaction, account_id)
        .await?
        .is_some()
    {
        enqueue_search(transaction, account_id, jobs::TraceContext::default()).await?;
    }
    Ok(())
}
