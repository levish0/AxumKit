use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::boards::create_board::create_board;
use super::boards::delete_board::delete_board;
use super::boards::get_board::get_board;
use super::boards::get_board_by_slug::get_board_by_slug;
use super::boards::get_boards::get_boards;
use super::boards::update_board::update_board;
use super::comments::create_comment::create_comment;
use super::comments::delete_comment::delete_comment;
use super::comments::get_comments::get_comments;
use super::comments::update_comment::update_comment;
use super::permissions::get_permissions;
use super::posts::create_post::create_post;
use super::posts::delete_post::delete_post;
use super::posts::get_post::get_post;
use super::posts::get_posts::get_posts;
use super::posts::lock_post::lock_post;
use super::posts::pin_post::pin_post;
use super::posts::reorder_pins::reorder_pins;
use super::posts::unlock_post::unlock_post;
use super::posts::unpin_post::unpin_post;
use super::posts::update_post::update_post;

pub fn board_routes() -> Router<AppState> {
    let protected_routes = Router::new()
        .route("/board", post(create_board))
        .route("/board/update", post(update_board))
        .route("/board/delete", post(delete_board))
        .route("/board/post", post(create_post))
        .route("/board/post/update", post(update_post))
        .route("/board/post/delete", post(delete_post))
        .route("/board/post/pin", post(pin_post))
        .route("/board/post/unpin", post(unpin_post))
        .route("/board/post/reorder-pins", post(reorder_pins))
        .route("/board/post/lock", post(lock_post))
        .route("/board/post/unlock", post(unlock_post))
        .route("/board/comment", post(create_comment))
        .route("/board/comment/update", post(update_comment))
        .route("/board/comment/delete", post(delete_comment));

    let public_routes = Router::new()
        .route("/board", get(get_board))
        .route("/board/by-slug", get(get_board_by_slug))
        .route("/board/list", get(get_boards))
        .route("/board/permissions", get(get_permissions))
        .route("/board/post", get(get_post))
        .route("/board/post/list", get(get_posts))
        .route("/board/comment/list", get(get_comments));

    protected_routes.merge(public_routes)
}
