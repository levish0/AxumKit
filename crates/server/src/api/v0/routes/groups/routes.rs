use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::create::create_group;
use super::delete::delete_group;
use super::list::list_groups;
use super::members::add::add_group_member;
use super::members::list::list_group_members;
use super::members::remove::remove_group_member;
use super::permissions::get::get_group_permissions;
use super::permissions::list_all::list_permissions;
use super::permissions::replace::replace_group_permissions;

pub fn group_routes() -> Router<AppState> {
    // Authorization lives in the service layer: reads require Mod, writes
    // require Admin (system groups are never mutable through this API).
    Router::new()
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/delete", post(delete_group))
        .route(
            "/groups/members",
            get(list_group_members).post(add_group_member),
        )
        .route("/groups/members/remove", post(remove_group_member))
        .route("/permissions", get(list_permissions))
        .route("/groups/permissions", get(get_group_permissions))
        .route(
            "/groups/permissions/replace",
            post(replace_group_permissions),
        )
}
