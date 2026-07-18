use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::posts::repository_get_board_post_by_id;
use crate::repository::board::repository_get_board_by_id;
use crate::service::actors::actor_response_by_id;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts;
use crate::service::board::mapper::{build_post_response, resolve_viewer_actor_id};
use dto::board::BoardPostResponse;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager as RedisClient;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

pub async fn service_get_board_post(
    db: &DatabaseConnection,
    redis_cache: &RedisClient,
    post_id: Uuid,
    session: Option<&SessionContext>,
    ip_address: &str,
) -> ServiceResult<BoardPostResponse> {
    let post = repository_get_board_post_by_id(db, post_id).await?;
    let board = repository_get_board_by_id(db, post.board_id).await?;

    let ctx = PermissionService::get_context(db, session).await?;
    let facts = load_board_facts(db, &board).await?;
    BoardPermission::View(facts.clone()).check(&ctx)?;

    // Best-effort view counting: never let a counting failure fail the read.
    // The returned `view_count` reflects the DB value before this view; the
    // buffered increment is flushed to the DB asynchronously by the worker.
    record_post_view(redis_cache, post_id, session, ip_address).await;

    let author = actor_response_by_id(db, post.actor_id).await?;
    let viewer_actor_id = resolve_viewer_actor_id(db, session).await?;

    Ok(build_post_response(
        &ctx,
        facts,
        post,
        author,
        viewer_actor_id,
    ))
}

/// Record a single view for `post_id`, deduplicated per viewer over a fixed
/// window. Authenticated callers are keyed by user id, anonymous ones by IP.
/// All Redis errors are swallowed (logged) so view counting can never break the
/// read path.
async fn record_post_view(
    redis_cache: &RedisClient,
    post_id: Uuid,
    session: Option<&SessionContext>,
    ip_address: &str,
) {
    let viewer = match session {
        Some(s) => format!("u:{}", s.user_id),
        None => format!("ip:{}", ip_address),
    };
    let dedup_key = constants::board_post_view_dedup_key(&post_id.to_string(), &viewer);

    // First view within the window? `SET NX EX` claims the dedup slot atomically.
    let claimed = match crate::utils::redis_cache::set_json_nx_with_ttl(
        redis_cache,
        &dedup_key,
        &true,
        constants::BOARD_POST_VIEW_DEDUP_TTL_SECONDS,
    )
    .await
    {
        Ok(claimed) => claimed,
        Err(e) => {
            tracing::warn!(error = ?e, %post_id, "board view dedup check failed; skipping count");
            return;
        }
    };

    if !claimed {
        return;
    }

    // Buffer the increment; the worker flushes pending deltas to the DB.
    if let Err(e) = crate::utils::redis_cache::hincr_by(
        redis_cache,
        constants::BOARD_POST_VIEW_PENDING_KEY,
        &post_id.to_string(),
        1,
    )
    .await
    {
        tracing::warn!(error = ?e, %post_id, "board view pending increment failed");
    }
}
