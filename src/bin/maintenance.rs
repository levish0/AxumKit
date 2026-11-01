use kit::{bootstrap::bootstrap, config::Settings, maintenance, observability, runtime};
use sentry::SentryFutureExt;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::from_environment()?;
    let _telemetry = observability::initialize(&settings.observability)?;
    let app = bootstrap(&settings).await?;
    let queue = app.queue.for_queue(maintenance::MAINTENANCE_QUEUE)?;
    let shutdown = CancellationToken::new();
    tracing::info!(
        queue = maintenance::MAINTENANCE_QUEUE,
        interval_seconds = settings.maintenance_interval_seconds,
        "maintenance started"
    );
    let watcher = tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            let _ = runtime::shutdown_signal().await;
            shutdown.cancel();
        }
    });
    let scheduler = maintenance::schedule(
        queue.clone(),
        settings.maintenance_interval_seconds,
        shutdown.clone(),
    );
    let worker = jobs::run(
        queue,
        jobs::WorkerSettings {
            concurrency: 1,
            ..settings.worker
        },
        shutdown.clone(),
        {
            let app = app.clone();
            let retention = settings.retention.clone();
            move |claim| {
                let app = app.clone();
                let retention = retention.clone();
                async move { maintenance::execute_job(&app, &retention, &claim).await }
                    .bind_hub(Arc::new(sentry::Hub::new_from_top(sentry::Hub::current())))
            }
        },
    );
    tokio::pin!(scheduler, worker);
    let result = tokio::select! {
        result = &mut scheduler => { shutdown.cancel(); result.and(worker.await) },
        result = &mut worker => { shutdown.cancel(); result.and(scheduler.await) },
    };
    watcher.abort();
    app.close().await?;
    Ok(result?)
}
