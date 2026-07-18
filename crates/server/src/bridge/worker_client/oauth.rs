use super::publish_job;
use crate::state::WorkerClient;
use errors::errors::Errors;
use job_queue::jobs::oauth::OAuthProfileImageJob;
use job_queue::subjects::OAUTH_PROFILE_IMAGE_SUBJECT;
use uuid::Uuid;

/// Queue OAuth profile image processing for a newly created user.
pub async fn process_oauth_profile_image(
    worker: &WorkerClient,
    user_id: Uuid,
    image_url: String,
) -> Result<(), Errors> {
    let job = OAuthProfileImageJob { user_id, image_url };

    publish_job(worker, OAUTH_PROFILE_IMAGE_SUBJECT, &job).await
}
