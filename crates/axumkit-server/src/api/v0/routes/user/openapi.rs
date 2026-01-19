use axumkit_dto::user::{
    CheckHandleAvailablePath, CheckHandleAvailableResponse, GetUserProfileByIdRequest,
    GetUserProfileRequest, PublicUserProfile, UpdateMyProfileRequest, UploadUserImageRequest,
    UploadUserImageResponse, UserResponse,
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
        )
    ),
    tags(
        (name = "User", description = "User endpoints")
    )
)]
pub struct UserApiDoc;
