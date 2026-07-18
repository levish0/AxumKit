use dto::user::UploadUserImageRequest;
use dto::user::{
    BanUserRequest, BanUserResponse, CheckHandleAvailablePath, CheckHandleAvailableResponse,
    ConfirmAccountDeletionRequest, DeleteMyAccountRequest, GetUserProfileByIdRequest,
    GetUserProfileRequest, GrantRoleRequest, GrantRoleResponse, PublicUserProfile,
    RevokeRoleRequest, RevokeRoleResponse, UnbanUserRequest, UnbanUserResponse,
    UpdateMyProfileRequest, UploadUserImageResponse, UserResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::profile::get_my_profile::get_my_profile,
        super::profile::update_my_profile::update_my_profile,
        super::profile::upload_profile_image::upload_profile_image,
        super::profile::upload_banner_image::upload_banner_image,
        super::profile::delete_profile_image::delete_profile_image,
        super::profile::delete_banner_image::delete_banner_image,
        super::account::delete_my_account::delete_my_account,
        super::account::confirm_account_deletion::confirm_account_deletion,
        super::public::get_user_profile::get_user_profile,
        super::public::get_user_profile_by_id::get_user_profile_by_id,
        super::account::check_handle_available::check_handle_available,
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
            DeleteMyAccountRequest,
            ConfirmAccountDeletionRequest,
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
        (name = "User Management", description = "User moderation endpoints")
    )
)]
pub struct UserApiDoc;
