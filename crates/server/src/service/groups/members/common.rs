use dto::groups::GroupMemberResponse;
use entity::group_members::Model as GroupMemberModel;

/// Maps a membership row to its API shape.
pub(super) fn member_to_response(member: GroupMemberModel) -> GroupMemberResponse {
    GroupMemberResponse {
        id: member.id,
        group_id: member.group_id,
        user_id: member.user_id,
        reason: member.reason,
        expires_at: member.expires_at,
        created_by: member.created_by,
        created_at: member.created_at,
    }
}
