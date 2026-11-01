use super::{comments, create_post, delete_post, get_post, list_posts, search_posts, update_post};
use crate::http::HttpState;

pub(in crate::http::v1) fn routes() -> axum::Router<HttpState> {
    axum::Router::new()
        .route(
            "/organizations/{organization_id}/posts",
            axum::routing::get(list_posts::list_posts).post(create_post::create_post),
        )
        .route(
            "/organizations/{organization_id}/posts/search",
            axum::routing::get(search_posts::search_posts),
        )
        .route(
            "/organizations/{organization_id}/posts/{post_id}",
            axum::routing::get(get_post::get_post)
                .put(update_post::update_post)
                .delete(delete_post::delete_post),
        )
        .route(
            "/organizations/{organization_id}/posts/{post_id}/comments",
            axum::routing::get(comments::list_comments::list_comments)
                .post(comments::create_comment::create_comment),
        )
        .route(
            "/organizations/{organization_id}/posts/{post_id}/comments/{comment_id}",
            axum::routing::delete(comments::delete_comment::delete_comment),
        )
}
