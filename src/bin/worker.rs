use kit::{bootstrap::bootstrap, config::Settings, observability, runtime};
use sentry::SentryFutureExt;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::from_environment()?;
    let _telemetry = observability::initialize(&settings.observability)?;
    let app = bootstrap(&settings).await?;
    let shutdown = CancellationToken::new();
    let watcher = tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            let _ = runtime::shutdown_signal().await;
            shutdown.cancel();
        }
    });
    let result = jobs::run(app.queue.clone(), settings.worker, shutdown, {
        let app = app.clone();
        move |claim| {
            let app = app.clone();
            async move { kit::job_handler::execute(&app, &claim).await }
                .bind_hub(Arc::new(sentry::Hub::new_from_top(sentry::Hub::current())))
        }
    })
    .await;
    watcher.abort();
    app.close().await?;
    Ok(result?)
}
