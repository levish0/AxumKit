use crate::bootstrap::Application;
use jobs::{Claim, HandlerFailure};

pub async fn execute(app: &Application, claim: &Claim) -> Result<(), HandlerFailure> {
    match claim.kind.as_str() {
        application::files::FILE_DELETE_JOB | application::files::FILE_SEARCH_JOB => {
            app.files.execute_job(claim).await
        }
        application::posts::POST_SEARCH_JOB => app.posts.execute_job(claim).await,
        application::profiles::PROFILE_SEARCH_JOB => app.profiles.execute_job(claim).await,
        application::notifications::FCM_DELIVERY_JOB => app.notifications.execute_job(claim).await,
        _ => app.identity.execute_job(claim, &app.mailer).await,
    }
}
