use super::publish_job;
use crate::state::WorkerClient;
use errors::errors::Errors;
use uuid::Uuid;
use worker::jobs::oauth::OAuthProfileImageJob;
use worker::nats::streams::OAUTH_PROFILE_IMAGE_SUBJECT;

/// Queue OAuth profile image processing for a newly created user.
pub async fn process_oauth_profile_image(
    worker: &WorkerClient,
    user_id: Uuid,
    image_url: String,
) -> Result<(), Errors> {
    let job = OAuthProfileImageJob { user_id, image_url };

    publish_job(worker, OAUTH_PROFILE_IMAGE_SUBJECT, &job).await
}
