use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::actors::repository_find_actors_by_ids;
use crate::repository::board::posts::{
    repository_delete_board_post, repository_get_board_post_by_id,
    repository_get_board_post_by_id_for_update,
};
use crate::service::auth::session_types::SessionContext;
use dto::board::DeleteBoardPostResponse;
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

pub async fn service_delete_board_post(
    db: &DatabaseConnection,
    post_id: Uuid,
    session: &SessionContext,
) -> ServiceResult<DeleteBoardPostResponse> {
    let post = repository_get_board_post_by_id(db, post_id).await?;

    let ctx = PermissionService::get_context(db, Some(session)).await?;

    // Owner can delete own post, moderators can delete any post.
    let is_owner = repository_find_actors_by_ids(db, &[post.actor_id])
        .await?
        .first()
        .is_some_and(|actor| actor.user_id == Some(session.user_id));
    BoardPermission::DeleteContent { is_owner }.check(&ctx)?;

    let txn = db.begin().await?;
    // Re-load under a row lock so concurrent deletes of the same post serialize: the
    // loser re-reads the now-deleted row and returns NotFound. Comments cascade away
    // via the FK, so no counter maintenance is needed here.
    repository_get_board_post_by_id_for_update(&txn, post_id).await?;
    repository_delete_board_post(&txn, post_id).await?;
    txn.commit().await?;

    info!(post_id = %post_id, "Board post deleted");

    Ok(DeleteBoardPostResponse { id: post_id })
}
