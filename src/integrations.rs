use crate::bootstrap::Application;
use application::ApplicationResult;

pub async fn initialize(app: &Application) -> ApplicationResult<()> {
    app.files.initialize_integrations().await?;
    app.posts.initialize_search().await?;
    app.profiles.initialize_search().await?;
    tracing::info!("configured integrations initialized");
    Ok(())
}

pub async fn repair_search(app: &Application) -> ApplicationResult<()> {
    let files = app.files.repair_search().await?;
    let posts = app.posts.repair_search().await?;
    let profiles = app.profiles.repair_search().await?;
    tracing::info!(files, posts, profiles, "search repair jobs registered");
    Ok(())
}
