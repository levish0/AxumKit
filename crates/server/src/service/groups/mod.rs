//! ACL management service layer.
//!
//! Provides ACL group, membership, and permission-grant administration use cases.

mod create_group;
mod delete_group;
mod list_groups;
mod members;
pub(crate) mod membership;
mod permissions;

pub use create_group::service_create_group;
pub use delete_group::service_delete_group;
pub use list_groups::service_list_groups;
pub use members::{
    service_add_group_member, service_list_group_members, service_remove_group_member,
};
pub use permissions::{
    service_get_group_permissions, service_list_permissions, service_replace_group_permissions,
};
