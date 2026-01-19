use super::create_post::create_post;
use super::delete_post::delete_post;
use super::get_post::get_post;
use super::list_posts::list_posts;
use super::update_post::update_post;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn posts_routes() -> Router<AppState> {
    Router::new()
        .route("/posts", post(create_post).get(list_posts))
        .route(
            "/posts/{id}",
            get(get_post).patch(update_post).delete(delete_post),
        )
}
