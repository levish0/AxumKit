use axumkit_dto::user::{
    BanUserRequest, BanUserResponse, CheckHandleAvailablePath, CheckHandleAvailableResponse,
    GetUserProfileByIdRequest, GetUserProfileRequest, GrantRoleRequest, GrantRoleResponse,
    PublicUserProfile, RevokeRoleRequest, RevokeRoleResponse, UnbanUserRequest, UnbanUserResponse,
    UpdateMyProfileRequest, UploadUserImageRequest, UploadUserImageResponse, UserResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::create_user::create_user,
        super::get_my_profile::get_my_profile,
        super::update_my_profile::update_my_profile,
        super::upload_profile_image::upload_profile_image,
        super::upload_banner_image::upload_banner_image,
        super::delete_profile_image::delete_profile_image,
        super::delete_banner_image::delete_banner_image,
        super::get_user_profile::get_user_profile,
        super::get_user_profile_by_id::get_user_profile_by_id,
        super::check_handle_available::check_handle_available,
        super::management::ban_user::ban_user,
        super::management::unban_user::unban_user,
        super::management::grant_role::grant_role,
        super::management::revoke_role::revoke_role,
    ),
    components(
        schemas(
            UserResponse,
            UpdateMyProfileRequest,
            UploadUserImageRequest,
            UploadUserImageResponse,
            GetUserProfileRequest,
            GetUserProfileByIdRequest,
            PublicUserProfile,
            CheckHandleAvailablePath,
            CheckHandleAvailableResponse,
            BanUserRequest,
            BanUserResponse,
            UnbanUserRequest,
            UnbanUserResponse,
            GrantRoleRequest,
            GrantRoleResponse,
            RevokeRoleRequest,
            RevokeRoleResponse,
        )
    ),
    tags(
        (name = "User", description = "User endpoints"),
        (name = "User Management", description = "User management endpoints (admin only)")
    )
)]
pub struct UserApiDoc;
