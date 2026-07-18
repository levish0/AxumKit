use dto::acl::{
    AclGroupListResponse, AclGroupMemberListResponse, AclGroupMemberResponse,
    AclGroupPermissionsResponse, AclGroupResponse, AclPermissionListResponse,
    AddAclGroupMemberRequest, CreateAclGroupRequest, DeleteAclGroupRequest,
    ListAclGroupMembersRequest, RemoveAclGroupMemberRequest, ReplaceAclGroupPermissionsRequest,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::groups::list::list_acl_groups,
        super::groups::create::create_acl_group,
        super::groups::delete::delete_acl_group,
        super::members::list::list_acl_group_members,
        super::members::add::add_acl_group_member,
        super::members::remove::remove_acl_group_member,
        super::permissions::list_all::list_permissions,
        super::permissions::get::get_acl_group_permissions,
        super::permissions::replace::replace_acl_group_permissions,
    ),
    components(
        schemas(
            CreateAclGroupRequest,
            DeleteAclGroupRequest,
            AclGroupResponse,
            AclGroupListResponse,
            AddAclGroupMemberRequest,
            RemoveAclGroupMemberRequest,
            ListAclGroupMembersRequest,
            AclGroupMemberResponse,
            AclGroupMemberListResponse,
            AclPermissionListResponse,
            AclGroupPermissionsResponse,
            ReplaceAclGroupPermissionsRequest,
        )
    ),
    tags(
        (name = "ACL", description = "ACL group, membership, and rule management")
    )
)]
pub struct AclApiDoc;
