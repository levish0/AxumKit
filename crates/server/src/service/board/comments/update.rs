use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::actors::{
    repository_find_actors_by_ids, repository_find_or_create_user_actor,
};
use crate::repository::board::comments::{
    repository_get_board_comment_by_id, repository_update_board_comment,
};
use crate::repository::board::posts::repository_get_board_post_by_id;
use crate::repository::board::repository_get_board_by_id;
use crate::repository::notification::NotificationTarget;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts;
use crate::service::notification::notify_mentions;
use crate::utils::mentions::resolve_mentions;
use crate::utils::session_helper::parse_attribution_ip;
use dto::board::{UpdateBoardCommentRequest, UpdateBoardCommentResponse};
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use std::collections::HashSet;
use tracing::info;
use uuid::Uuid;

pub async fn service_update_board_comment(
    db: &DatabaseConnection,
    payload: UpdateBoardCommentRequest,
    session: &SessionContext,
    ip_address: &str,
) -> ServiceResult<UpdateBoardCommentResponse> {
    let comment = repository_get_board_comment_by_id(db, payload.comment_id).await?;
    let post = repository_get_board_post_by_id(db, comment.post_id).await?;
    let board = repository_get_board_by_id(db, post.board_id).await?;

    let ctx = PermissionService::get_context(db, Some(session)).await?;

    // Content edits are owner-only (moderators sanction, they don't rewrite).
    let is_owner = repository_find_actors_by_ids(db, &[comment.actor_id])
        .await?
        .first()
        .is_some_and(|actor| actor.user_id == Some(session.user_id));
    let facts = load_board_facts(db, &board).await?;
    BoardPermission::EditContent { is_owner, facts }.check(&ctx)?;

    // Notify only users newly mentioned by this edit — mentions present in the
    // new content but not the previous one — so re-saving a comment does not
    // spam every already-mentioned user again.
    let new_mentions: HashSet<Uuid> = resolve_mentions(db, &payload.content)
        .await?
        .into_iter()
        .collect();
    let old_mentions: HashSet<Uuid> = resolve_mentions(db, &comment.content)
        .await?
        .into_iter()
        .collect();
    let mentioned_user_ids: Vec<Uuid> = new_mentions.difference(&old_mentions).copied().collect();

    let txn = db.begin().await?;
    let actor = repository_find_or_create_user_actor(&txn, session.user_id).await?;

    let updated =
        repository_update_board_comment(&txn, payload.comment_id, payload.content).await?;

    txn.commit().await?;

    // Notify only newly-mentioned users (best-effort). Deep-link targets the comment.
    let actor_ip = Some(parse_attribution_ip(ip_address)?);
    notify_mentions(
        db,
        mentioned_user_ids,
        Some(session.user_id),
        actor.id,
        actor_ip,
        NotificationTarget::BoardComment {
            board_id: post.board_id,
            post_id: post.id,
            comment_id: updated.id,
        },
        json!({
            "board_name": board.name,
            "board_slug": board.slug,
            "post_title": post.title,
        }),
    )
    .await;

    info!(comment_id = %updated.id, "Board comment updated");

    Ok(UpdateBoardCommentResponse { id: updated.id })
}
