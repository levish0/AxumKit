use entity::board_posts::{Column as PostColumn, Entity as PostEntity};
use errors::errors::Errors;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Rewrite a board's pin order, assigning positions `0..n` from `post_ids`.
///
/// Positions come from the list index rather than the caller, mirroring
/// `repository_replace_acl_rules` — a client sends the order it wants, never the
/// numbers. This also renormalizes whatever sparse or negative slots the pin
/// path left behind.
///
/// Run inside a transaction: the rewrite is all-or-nothing. The caller is
/// expected to have already checked that `post_ids` is exactly the board's
/// current pin set, so a missing row here would be a bug, not a stale client.
pub async fn repository_reorder_board_pins<C>(conn: &C, post_ids: &[Uuid]) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    for (position, post_id) in post_ids.iter().enumerate() {
        let updated = PostEntity::update_many()
            .col_expr(PostColumn::PinnedPosition, Expr::value(position as i32))
            .filter(PostColumn::Id.eq(*post_id))
            .exec(conn)
            .await?;

        if updated.rows_affected == 0 {
            return Err(Errors::BoardPostNotFound);
        }
    }

    Ok(())
}
