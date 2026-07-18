use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::actors::repository_find_actor_by_id;
use crate::repository::actors::repository_find_or_create_user_actor;
use crate::repository::board::comments::{
    repository_create_board_comment, repository_get_board_comment_by_id,
    repository_increment_comment_reply_count,
};
use crate::repository::board::posts::{
    repository_get_board_post_by_id, repository_increment_post_comment_count,
};
use crate::repository::board::repository_get_board_by_id;
use crate::repository::notification::NotificationTarget;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts;
use crate::service::notification::{notify_mentions, service_notify_user};
use crate::utils::mentions::resolve_mentions;
use crate::utils::session_helper::parse_attribution_ip;
use dto::board::{CreateBoardCommentRequest, CreateBoardCommentResponse};
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

pub async fn service_create_board_comment(
    db: &DatabaseConnection,
    payload: CreateBoardCommentRequest,
    session: &SessionContext,
    ip_address: &str,
) -> ServiceResult<CreateBoardCommentResponse> {
    let post = repository_get_board_post_by_id(db, payload.post_id).await?;
    let board = repository_get_board_by_id(db, post.board_id).await?;

    let ctx = PermissionService::get_context(db, Some(session)).await?;
    let facts = load_board_facts(db, &board).await?;
    BoardPermission::Write(facts).check(&ctx)?;

    if post.is_locked {
        return Err(Errors::BoardPostLocked);
    }

    // Depth cap at 2 (YouTube-style): a reply to a reply attaches to the same thread
    // root, so a stored reply's parent_comment_id always points at a top-level comment.
    let resolved_parent_id: Option<Uuid> = match payload.parent_comment_id {
        Some(pid) => {
            let parent = repository_get_board_comment_by_id(db, pid).await?;
            if parent.post_id != payload.post_id {
                return Err(Errors::BoardCommentNotFound);
            }
            Some(parent.parent_comment_id.unwrap_or(parent.id))
        }
        None => None,
    };

    // Resolve @handle mentions before opening the transaction
    let mentioned_user_ids: Vec<Uuid> = resolve_mentions(db, &payload.content).await?;

    let txn = db.begin().await?;
    let actor = repository_find_or_create_user_actor(&txn, session.user_id).await?;

    let comment = repository_create_board_comment(
        &txn,
        payload.post_id,
        actor.id,
        resolved_parent_id,
        payload.content,
    )
    .await?;

    repository_increment_post_comment_count(&txn, payload.post_id).await?;
    if let Some(root_id) = resolved_parent_id {
        repository_increment_comment_reply_count(&txn, root_id).await?;
    }

    txn.commit().await?;

    // Create mention notifications (best-effort). Deep-link targets the comment.
    let actor_ip = Some(parse_attribution_ip(ip_address)?);
    notify_mentions(
        db,
        mentioned_user_ids.clone(),
        Some(session.user_id),
        actor.id,
        actor_ip,
        NotificationTarget::BoardComment {
            board_id: post.board_id,
            post_id: post.id,
            comment_id: comment.id,
        },
        json!({
            "board_name": board.name,
            "board_slug": board.slug,
            "post_title": post.title,
        }),
    )
    .await;

    // Alert the post author about the new comment (best-effort; skips
    // self-comments and respects the author's action preference).
    if let Some(post_author) = repository_find_actor_by_id(db, post.actor_id)
        .await
        .ok()
        .flatten()
        .and_then(|actor| actor.user_id)
        && post_author != session.user_id
        && !mentioned_user_ids.contains(&post_author)
    {
        let _ = service_notify_user(
            db,
            post_author,
            Some(actor.id),
            actor_ip,
            entity::common::NotificationType::Board,
            constants::NotificationAction::BoardCommentCreated,
            NotificationTarget::BoardComment {
                board_id: post.board_id,
                post_id: post.id,
                comment_id: comment.id,
            },
            json!({
                "board_name": board.name,
                "board_slug": board.slug,
                "post_title": post.title,
            }),
        )
        .await;
    }

    info!(comment_id = %comment.id, post_id = %payload.post_id, "Board comment created");

    Ok(CreateBoardCommentResponse { id: comment.id })
}
