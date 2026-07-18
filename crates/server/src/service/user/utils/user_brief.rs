use crate::service::user::mapper::mask_user_identity;
use dto::user::UserBriefResponse;
use entity::users::Model as UserModel;

pub fn user_brief_response(user: &UserModel) -> UserBriefResponse {
    let identity = mask_user_identity(user);

    UserBriefResponse {
        id: identity.id,
        handle: identity.handle,
        display_name: identity.display_name,
        profile_image: identity.profile_image,
        deactivated: identity.deactivated,
    }
}
