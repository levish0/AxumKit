use super::{get_own_profile, get_user, list_users, save_profile, search_users};
use crate::http::HttpState;
use axum::routing::get;

pub(in crate::http::v1) fn routes() -> axum::Router<HttpState> {
    axum::Router::new()
        .route(
            "/account/profile",
            get(get_own_profile::get_own_profile).put(save_profile::save_profile),
        )
        .route(
            "/organizations/{organization_id}/users",
            get(list_users::list_users),
        )
        .route(
            "/organizations/{organization_id}/users/search",
            get(search_users::search_users),
        )
        .route(
            "/organizations/{organization_id}/users/{account_id}",
            get(get_user::get_user),
        )
}
