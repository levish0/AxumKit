//! ACL management service layer.
//!
//! Provides ACL group, membership, and permission-grant administration use cases.

mod groups;
mod members;
pub(crate) mod membership;
mod permissions;

pub use groups::{service_create_acl_group, service_delete_acl_group, service_list_acl_groups};
pub use members::{
    service_add_acl_group_member, service_list_acl_group_members, service_remove_acl_group_member,
};
pub use permissions::{
    service_get_acl_group_permissions, service_list_permissions,
    service_replace_acl_group_permissions,
};
