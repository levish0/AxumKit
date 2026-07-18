use crate::{CacheClient, DbPool};
use entity::board_posts::{Column as PostColumn, Entity as PostEntity};
use redis::Script;
use sea_orm::{ColumnTrait, EntityTrait, ExprTrait, QueryFilter};
use std::sync::LazyLock;
use uuid::Uuid;

static DRAIN_HASH_SCRIPT: LazyLock<Script> =
    LazyLock::new(|| Script::new(include_str!("lua/drain_hash.lua")));

/// Drain buffered board-post view deltas from Redis and apply them to the DB.
///
/// The pending counts live in a volatile Redis hash
/// (`board:post:view:pending`) written by the server on each counted view. We
/// atomically read-and-clear the hash (so concurrent worker instances don't
/// double-apply), then add each delta to `board_posts.view_count`. Best-effort:
/// a drained-but-unapplied delta is lost on failure, which is acceptable for
/// view counts.
pub async fn run_flush_board_view_counts(db: &DbPool, cache: &CacheClient) {
    match flush(db, cache).await {
        Ok(0) => {}
        Ok(updated) => tracing::info!(updated_posts = updated, "Flushed board view counts"),
        Err(e) => tracing::error!(error = %e, "Failed to flush board view counts"),
    }
}

async fn flush(db: &DbPool, cache: &CacheClient) -> Result<usize, anyhow::Error> {
    let mut conn = cache.as_ref().clone();
    // Atomic HGETALL + DEL → flat [field, value, field, value, ...].
    let pairs: Vec<String> = DRAIN_HASH_SCRIPT
        .key(constants::BOARD_POST_VIEW_PENDING_KEY)
        .invoke_async(&mut conn)
        .await?;

    if pairs.is_empty() {
        return Ok(0);
    }

    let mut updated = 0usize;
    for chunk in pairs.chunks(2) {
        let [post_id_str, delta_str] = chunk else {
            continue;
        };

        let post_id = match Uuid::parse_str(post_id_str) {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!(error = %e, post_id = %post_id_str, "Skipping invalid post id in view flush");
                continue;
            }
        };
        let delta: i64 = match delta_str.parse() {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(error = %e, delta = %delta_str, "Skipping invalid delta in view flush");
                continue;
            }
        };
        if delta == 0 {
            continue;
        }

        PostEntity::update_many()
            .filter(PostColumn::Id.eq(post_id))
            .col_expr(
                PostColumn::ViewCount,
                sea_orm::sea_query::Expr::col(PostColumn::ViewCount).add(delta),
            )
            .exec(db.as_ref())
            .await?;
        updated += 1;
    }

    Ok(updated)
}
