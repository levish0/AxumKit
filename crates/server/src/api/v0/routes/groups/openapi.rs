use dto::groups::{
    AddGroupMemberRequest, CreateGroupRequest, DeleteGroupRequest, GroupListResponse,
    GroupMemberListResponse, GroupMemberResponse, GroupPermissionsResponse, GroupResponse,
    ListGroupMembersRequest, PermissionListResponse, RemoveGroupMemberRequest,
    ReplaceGroupPermissionsRequest,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::list::list_groups,
        super::create::create_group,
        super::delete::delete_group,
        super::members::list::list_group_members,
        super::members::add::add_group_member,
        super::members::remove::remove_group_member,
        super::permissions::list_all::list_permissions,
        super::permissions::get::get_group_permissions,
        super::permissions::replace::replace_group_permissions,
    ),
    components(
        schemas(
            CreateGroupRequest,
            DeleteGroupRequest,
            GroupResponse,
            GroupListResponse,
            AddGroupMemberRequest,
            RemoveGroupMemberRequest,
            ListGroupMembersRequest,
            GroupMemberResponse,
            GroupMemberListResponse,
            PermissionListResponse,
            GroupPermissionsResponse,
            ReplaceGroupPermissionsRequest,
        )
    ),
    tags(
        (name = "ACL", description = "ACL group, membership, and rule management")
    )
)]
pub struct GroupsApiDoc;
