use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::comments::{
    repository_exists_newer_board_comment, repository_exists_older_board_comment,
    repository_find_board_comments, repository_find_board_comments_around,
    repository_get_board_comment_by_id,
};
use crate::repository::board::posts::repository_get_board_post_by_id;
use crate::repository::board::repository_get_board_by_id;
use crate::service::actors::actor_response_map;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts;
use crate::service::board::mapper::{build_comment_response, resolve_viewer_actor_id};
use crate::service::cursor_pagination::{cursor_flags, reverse_if_older};
use dto::board::{BoardCommentListResponse, BoardCommentResponse, GetBoardCommentsRequest};
use dto::pagination::CursorDirection;
use errors::errors::{Errors, ServiceResult};
use sea_orm::DatabaseConnection;
use std::collections::HashSet;

pub async fn service_list_board_comments(
    db: &DatabaseConnection,
    payload: GetBoardCommentsRequest,
    session: Option<&SessionContext>,
) -> ServiceResult<BoardCommentListResponse> {
    let post = repository_get_board_post_by_id(db, payload.post_id).await?;
    let board = repository_get_board_by_id(db, post.board_id).await?;

    let ctx = PermissionService::get_context(db, session).await?;
    let facts = load_board_facts(db, &board).await?;
    BoardPermission::View(facts.clone()).check(&ctx)?;

    // A focus (deep-link) load returns the ascending window around the anchor,
    // so `cursor_direction` only matters for cursor paging.
    let is_older = payload.focus_comment_id.is_none()
        && payload.cursor_direction == Some(CursorDirection::Older);
    // This list displays oldest-first (ascending), unlike the desc-default lists
    // the cursor helper assumes. The no-cursor and Newer pages are already
    // ascending; only the Older page is fetched descending, so the display reverse
    // below normalizes the Older page back to ascending — every page is returned
    // ascending regardless of direction.
    let page_ascending = !is_older;

    let mut comments = if let Some(focus_id) = payload.focus_comment_id {
        if payload.cursor_id.is_some() {
            return Err(Errors::ValidationError(
                "focus_comment_id cannot be combined with cursor_id.".to_string(),
            ));
        }
        // The anchor must sit in the requested listing scope: same post, and the
        // same thread (top-level vs. a specific comment's replies).
        let focus = repository_get_board_comment_by_id(db, focus_id).await?;
        if focus.post_id != payload.post_id || focus.parent_comment_id != payload.parent_comment_id
        {
            return Err(Errors::BoardCommentNotFound);
        }
        repository_find_board_comments_around(
            db,
            payload.post_id,
            payload.parent_comment_id,
            focus,
            payload.limit,
        )
        .await?
    } else {
        repository_find_board_comments(
            db,
            payload.post_id,
            payload.parent_comment_id,
            payload.cursor_id,
            payload.cursor_direction,
            payload.limit,
        )
        .await?
    };

    let (has_newer, has_older) = cursor_flags(
        &comments,
        page_ascending,
        |comment| comment.id,
        |cursor| {
            repository_exists_newer_board_comment(
                db,
                payload.post_id,
                payload.parent_comment_id,
                cursor,
            )
        },
        |cursor| {
            repository_exists_older_board_comment(
                db,
                payload.post_id,
                payload.parent_comment_id,
                cursor,
            )
        },
    )
    .await?;

    reverse_if_older(&mut comments, is_older);

    let actor_ids: Vec<_> = comments
        .iter()
        .map(|comment| comment.actor_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let authors = actor_response_map(db, &actor_ids).await?;
    let viewer_actor_id = resolve_viewer_actor_id(db, session).await?;

    let data: Vec<BoardCommentResponse> = comments
        .into_iter()
        .map(|comment| {
            let author = authors.get(&comment.actor_id).cloned();
            build_comment_response(&ctx, facts.clone(), comment, author, viewer_actor_id)
        })
        .collect();

    Ok(BoardCommentListResponse {
        data,
        has_newer,
        has_older,
    })
}
