use crate::repository::user::get_by_id::repository_get_user_by_id;
use axumkit_dto::user::PublicUserProfile;
use axumkit_errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

pub async fn service_get_user_profile_by_id(
    conn: &DatabaseConnection,
    user_id: Uuid,
) -> ServiceResult<PublicUserProfile> {
    let user = repository_get_user_by_id(conn, user_id).await?;

    let profile = PublicUserProfile {
        id: user.id,
        handle: user.handle,
        display_name: user.display_name,
        bio: user.bio,
        profile_image: user.profile_image,
        banner_image: user.banner_image,
        is_verified: user.verified_at.is_some(),
        created_at: user.created_at,
    };

    Ok(profile)
}
