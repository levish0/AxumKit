use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::actors::repository_find_actors_by_ids;
use crate::repository::board::comments::{
    repository_decrement_comment_reply_count, repository_delete_board_comment,
    repository_get_board_comment_by_id, repository_get_board_comment_by_id_for_update,
};
use crate::repository::board::posts::repository_decrement_post_comment_count;
use crate::service::auth::session_types::SessionContext;
use dto::board::DeleteBoardCommentResponse;
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

pub async fn service_delete_board_comment(
    db: &DatabaseConnection,
    comment_id: Uuid,
    session: &SessionContext,
) -> ServiceResult<DeleteBoardCommentResponse> {
    let comment = repository_get_board_comment_by_id(db, comment_id).await?;

    let ctx = PermissionService::get_context(db, Some(session)).await?;

    // Owner can delete own comment, moderators can delete any comment.
    let is_owner = repository_find_actors_by_ids(db, &[comment.actor_id])
        .await?
        .first()
        .is_some_and(|actor| actor.user_id == Some(session.user_id));
    BoardPermission::DeleteContent { is_owner }.check(&ctx)?;

    let txn = db.begin().await?;
    // Lock parent-before-child: a concurrent delete of the thread root locks the root and
    // then cascade-locks this reply, so acquire the root first here to keep a consistent
    // lock order and avoid a deadlock. parent_comment_id is immutable, so deciding the
    // order from the unlocked read above is safe.
    if let Some(root_id) = comment.parent_comment_id {
        repository_get_board_comment_by_id_for_update(&txn, root_id).await?;
    }
    // Re-load the target under a row lock so concurrent deletes of the same comment
    // serialize: the loser re-reads the now-deleted row and returns NotFound, so the
    // post's comment_count and the thread root's reply_count are each adjusted exactly once.
    let locked = repository_get_board_comment_by_id_for_update(&txn, comment_id).await?;

    match locked.parent_comment_id {
        // Top-level comment: its replies cascade away, so drop the comment plus all of
        // them from the post's comment_count in one go.
        None => {
            repository_decrement_post_comment_count(&txn, locked.post_id, 1 + locked.reply_count)
                .await?;
        }
        // Reply: drop one from the post's comment_count and one from its root's reply_count.
        Some(root_id) => {
            repository_decrement_post_comment_count(&txn, locked.post_id, 1).await?;
            repository_decrement_comment_reply_count(&txn, root_id).await?;
        }
    }

    repository_delete_board_comment(&txn, comment_id).await?;
    txn.commit().await?;

    info!(comment_id = %comment_id, "Board comment deleted");

    Ok(DeleteBoardCommentResponse { id: comment_id })
}
