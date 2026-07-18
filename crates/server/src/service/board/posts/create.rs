use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::actors::repository_find_or_create_user_actor;
use crate::repository::board::posts::repository_create_board_post;
use crate::repository::board::repository_get_board_by_id;
use crate::repository::notification::NotificationTarget;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts;
use crate::service::notification::notify_mentions;
use crate::utils::mentions::resolve_mentions;
use crate::utils::session_helper::parse_attribution_ip;
use dto::board::{CreateBoardPostRequest, CreateBoardPostResponse};
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

pub async fn service_create_board_post(
    db: &DatabaseConnection,
    payload: CreateBoardPostRequest,
    session: &SessionContext,
    ip_address: &str,
) -> ServiceResult<CreateBoardPostResponse> {
    let board = repository_get_board_by_id(db, payload.board_id).await?;

    let ctx = PermissionService::get_context(db, Some(session)).await?;
    let facts = load_board_facts(db, &board).await?;
    BoardPermission::Write(facts).check(&ctx)?;

    // Resolve @handle mentions before opening the transaction
    let mentioned_user_ids: Vec<Uuid> = resolve_mentions(db, &payload.content).await?;

    let txn = db.begin().await?;
    let actor = repository_find_or_create_user_actor(&txn, session.user_id).await?;

    let post = repository_create_board_post(
        &txn,
        payload.board_id,
        actor.id,
        payload.title,
        payload.content,
    )
    .await?;

    txn.commit().await?;

    // Create mention notifications (best-effort)
    let actor_ip = Some(parse_attribution_ip(ip_address)?);
    notify_mentions(
        db,
        mentioned_user_ids,
        Some(session.user_id),
        actor.id,
        actor_ip,
        NotificationTarget::BoardPost {
            board_id: payload.board_id,
            post_id: post.id,
        },
        json!({
            "board_name": board.name,
            "board_slug": board.slug,
            "post_title": post.title,
        }),
    )
    .await;

    info!(post_id = %post.id, board_id = %payload.board_id, "Board post created");

    Ok(CreateBoardPostResponse { id: post.id })
}
