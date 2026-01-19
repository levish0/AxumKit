use crate::repository::user::{UserUpdateParams, repository_update_user};
use crate::service::auth::session_types::SessionContext;
use axumkit_dto::user::UserResponse;
use axumkit_dto::user::request::UpdateMyProfileRequest;
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;

pub async fn service_update_my_profile(
    conn: &DatabaseConnection,
    session: &SessionContext,
    request: UpdateMyProfileRequest,
) -> Result<UserResponse, Errors> {
    let params = UserUpdateParams {
        display_name: request.display_name,
        bio: request.bio.map(Some),
        ..Default::default()
    };

    let updated_user = repository_update_user(conn, session.user_id, params).await?;

    Ok(UserResponse {
        id: session.user_id,
        email: updated_user.email,
        handle: updated_user.handle,
        display_name: updated_user.display_name,
        bio: updated_user.bio,
        profile_image: updated_user.profile_image,
        banner_image: updated_user.banner_image,
        is_verified: updated_user.verified_at.is_some(),
        created_at: updated_user.created_at,
    })
}
