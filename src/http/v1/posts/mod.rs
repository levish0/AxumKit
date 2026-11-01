pub(in crate::http::v1) mod comments;
pub(in crate::http::v1) mod contracts;
pub(in crate::http::v1) mod create_post;
pub(in crate::http::v1) mod delete_post;
pub(in crate::http::v1) mod get_post;
pub(in crate::http::v1) mod list_posts;
mod routes;
pub(in crate::http::v1) mod search_posts;
pub(in crate::http::v1) mod update_post;

pub(super) use routes::routes;
