use crate::repository::user::repository_get_user_by_id;
use crate::service::auth::session_types::SessionContext;
use crate::utils::r2_url::build_r2_public_url;
use axumkit_dto::user::UserResponse;
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;

pub async fn service_get_my_profile(
    conn: &DatabaseConnection,
    session: &SessionContext,
) -> Result<UserResponse, Errors> {
    let user = repository_get_user_by_id(conn, session.user_id).await?;

    let response = UserResponse {
        id: session.user_id,
        email: user.email,
        handle: user.handle,
        display_name: user.display_name,
        bio: user.bio,
        profile_image: user.profile_image.as_deref().map(build_r2_public_url),
        banner_image: user.banner_image.as_deref().map(build_r2_public_url),
        is_verified: user.verified_at.is_some(),
        created_at: user.created_at,
    };

    Ok(response)
}
