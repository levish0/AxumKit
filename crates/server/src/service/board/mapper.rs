use crate::permission::UserContext;
use crate::permission::board::{BoardFacts, BoardPermission};
use crate::permission::rule::Rule;
use crate::repository::actors::repository_find_actor_by_user_id;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::permissions::build_board_permissions_response;
use dto::actor::ActorResponse;
use dto::board::{BoardCommentResponse, BoardPostResponse, BoardResponse};
use entity::board_comments::Model as BoardCommentModel;
use entity::board_posts::Model as BoardPostModel;
use entity::boards::Model as BoardModel;
use errors::errors::ServiceResult;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

/// Resolves the caller's own actor id, used to decide content ownership.
///
/// A user owns exactly one user-kind actor (1:1), so comparing a post/comment's
/// `actor_id` against this id is equivalent to the `actor.user_id == session.user_id`
/// check enforced on the mutation paths. Returns `None` for anonymous callers or a
/// user who has never acted (and therefore owns nothing).
pub async fn resolve_viewer_actor_id<C>(
    conn: &C,
    session: Option<&SessionContext>,
) -> ServiceResult<Option<Uuid>>
where
    C: ConnectionTrait,
{
    let Some(session) = session else {
        return Ok(None);
    };

    Ok(repository_find_actor_by_user_id(conn, session.user_id)
        .await?
        .map(|actor| actor.id))
}

/// Maps a board entity to its response, stamping the caller's capability flags.
///
/// Board-level flags are computed by [`build_board_permissions_response`] so the
/// embedded flags stay in lockstep with the standalone permissions endpoint.
/// `facts` carry the board's already-resolved ACL chain.
pub fn build_board_response(
    ctx: &UserContext,
    board: BoardModel,
    facts: BoardFacts,
) -> BoardResponse {
    let perms = build_board_permissions_response(ctx, facts);

    BoardResponse {
        id: board.id,
        slug: board.slug,
        name: board.name,
        description: board.description,
        order: board.order,
        is_disabled: board.is_disabled,
        can_write: perms.can_write,
        can_moderate: perms.can_moderate,
        created_at: board.created_at,
        updated_at: board.updated_at,
    }
}

/// Maps a board post entity to its response, stamping the caller's capability flags.
///
/// `facts` are the parent board's facts (needed for the author edit path) and
/// `viewer_actor_id` is the caller's own actor id from [`resolve_viewer_actor_id`].
pub fn build_post_response(
    ctx: &UserContext,
    facts: BoardFacts,
    post: BoardPostModel,
    author: Option<ActorResponse>,
    viewer_actor_id: Option<Uuid>,
) -> BoardPostResponse {
    let is_owner = viewer_actor_id == Some(post.actor_id);

    BoardPostResponse {
        id: post.id,
        board_id: post.board_id,
        author_actor_id: post.actor_id,
        author,
        title: post.title,
        content: post.content,
        is_pinned: post.pinned_position.is_some(),
        is_locked: post.is_locked,
        view_count: post.view_count,
        comment_count: post.comment_count,
        can_edit: BoardPermission::EditContent { is_owner, facts }.is_allowed(ctx),
        can_delete: BoardPermission::DeleteContent { is_owner }.is_allowed(ctx),
        created_at: post.created_at,
        edited_at: post.edited_at,
    }
}

/// Maps a board comment entity to its response, stamping the caller's capability flags.
pub fn build_comment_response(
    ctx: &UserContext,
    facts: BoardFacts,
    comment: BoardCommentModel,
    author: Option<ActorResponse>,
    viewer_actor_id: Option<Uuid>,
) -> BoardCommentResponse {
    let is_owner = viewer_actor_id == Some(comment.actor_id);

    BoardCommentResponse {
        id: comment.id,
        post_id: comment.post_id,
        parent_comment_id: comment.parent_comment_id,
        author_actor_id: comment.actor_id,
        author,
        content: comment.content,
        reply_count: comment.reply_count,
        can_edit: BoardPermission::EditContent { is_owner, facts }.is_allowed(ctx),
        can_delete: BoardPermission::DeleteContent { is_owner }.is_allowed(ctx),
        created_at: comment.created_at,
        edited_at: comment.edited_at,
    }
}
