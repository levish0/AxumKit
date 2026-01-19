use crate::connection::r2_conn::R2Client;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::service::auth::session_types::SessionContext;
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;
use tracing::{info, warn};

pub async fn service_delete_profile_image(
    conn: &DatabaseConnection,
    r2_client: &R2Client,
    session: &SessionContext,
) -> Result<(), Errors> {
    let user = repository_get_user_by_id(conn, session.user_id).await?;

    let Some(storage_key) = user.profile_image else {
        return Err(Errors::NotFound("No profile image to delete".to_string()));
    };

    // Delete from R2 (best effort - don't fail if R2 delete fails)
    if let Err(e) = r2_client.delete(&storage_key).await {
        warn!(
            "Failed to delete profile image from R2: {}. Continuing with DB update.",
            e
        );
    }

    repository_update_user(
        conn,
        session.user_id,
        UserUpdateParams {
            profile_image: Some(None),
            ..Default::default()
        },
    )
    .await?;

    info!("Profile image deleted for user: {}", session.user_id);

    Ok(())
}
