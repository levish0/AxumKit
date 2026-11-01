pub(in crate::http::v1) mod contracts;
pub(in crate::http::v1) mod get_own_profile;
pub(in crate::http::v1) mod get_user;
pub(in crate::http::v1) mod list_users;
mod routes;
pub(in crate::http::v1) mod save_profile;
pub(in crate::http::v1) mod search_users;

pub(super) use routes::routes;
