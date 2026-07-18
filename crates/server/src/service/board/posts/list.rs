use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::posts::{
    repository_find_board_posts, repository_find_pinned_board_posts,
};
use crate::repository::board::repository_get_board_by_id;
use crate::service::actors::actor_response_map;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts;
use crate::service::board::mapper::{build_post_response, resolve_viewer_actor_id};
use dto::board::{BoardPostListResponse, BoardPostResponse, GetBoardPostsRequest};
use entity::board_posts::Model as BoardPostModel;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use std::collections::HashSet;

pub async fn service_list_board_posts(
    db: &DatabaseConnection,
    payload: GetBoardPostsRequest,
    session: Option<&SessionContext>,
) -> ServiceResult<BoardPostListResponse> {
    let board = repository_get_board_by_id(db, payload.board_id).await?;

    let ctx = PermissionService::get_context(db, session).await?;
    let facts = load_board_facts(db, &board).await?;
    BoardPermission::View(facts.clone()).check(&ctx)?;

    let page = payload.page;
    let page_size = payload.page_size;

    let offset = (page as u64 - 1) * page_size as u64;
    let limit = page_size as u64 + 1;

    // Pins are read whole and served above every page, so they are paginated by
    // neither `offset` nor `has_more` — the paged query excludes them entirely.
    // Folding them into that one window instead is what used to strand them on
    // page 1 and eat its slots.
    let pinned = repository_find_pinned_board_posts(db, payload.board_id).await?;
    let mut posts = repository_find_board_posts(db, payload.board_id, offset, limit).await?;

    let has_more = posts.len() > page_size as usize;
    if has_more {
        posts.pop();
    }

    let actor_ids: Vec<_> = pinned
        .iter()
        .chain(posts.iter())
        .map(|post| post.actor_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let authors = actor_response_map(db, &actor_ids).await?;
    let viewer_actor_id = resolve_viewer_actor_id(db, session).await?;

    // The list view shows titles/metadata, not post bodies, so it does not render
    // or cache content (`rendered_content = None`); the detail read renders and
    // caches on demand.
    let build = |post: BoardPostModel| {
        let author = authors.get(&post.actor_id).cloned();
        build_post_response(&ctx, facts.clone(), post, author, viewer_actor_id)
    };

    let pinned_responses: Vec<BoardPostResponse> = pinned.into_iter().map(&build).collect();
    let post_responses: Vec<BoardPostResponse> = posts.into_iter().map(&build).collect();

    Ok(BoardPostListResponse {
        pinned: pinned_responses,
        posts: post_responses,
        current_page: page,
        page_size,
        has_more,
    })
}
