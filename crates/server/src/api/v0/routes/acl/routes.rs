use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::groups::create::create_acl_group;
use super::groups::delete::delete_acl_group;
use super::groups::list::list_acl_groups;
use super::members::add::add_acl_group_member;
use super::members::list::list_acl_group_members;
use super::members::remove::remove_acl_group_member;
use super::permissions::get::get_acl_group_permissions;
use super::permissions::list_all::list_permissions;
use super::permissions::replace::replace_acl_group_permissions;

pub fn acl_routes() -> Router<AppState> {
    // Authorization lives in the service layer: reads require Mod, writes
    // require Admin (system groups are never mutable through this API).
    Router::new()
        .route("/acl/groups", get(list_acl_groups).post(create_acl_group))
        .route("/acl/groups/delete", post(delete_acl_group))
        .route(
            "/acl/groups/members",
            get(list_acl_group_members).post(add_acl_group_member),
        )
        .route("/acl/groups/members/remove", post(remove_acl_group_member))
        .route("/acl/permissions", get(list_permissions))
        .route("/acl/groups/permissions", get(get_acl_group_permissions))
        .route(
            "/acl/groups/permissions/replace",
            post(replace_acl_group_permissions),
        )
}
