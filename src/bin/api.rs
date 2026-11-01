use kit::{bootstrap::bootstrap, config::Settings, http::build_router, observability, runtime};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::from_environment()?;
    let _telemetry = observability::initialize(&settings.observability)?;
    let app = bootstrap(&settings).await?;
    let listener = TcpListener::bind(settings.http.bind_address).await?;
    tracing::info!(address=%listener.local_addr()?, "API listening");
    let shutdown = CancellationToken::new();
    let watcher = tokio::spawn({
        let shutdown = shutdown.clone();
        let app = app.clone();
        async move {
            let _ = runtime::shutdown_signal().await;
            app.begin_shutdown();
            shutdown.cancel();
        }
    });
    let result = runtime::serve(listener, build_router(app.clone(), settings.http), shutdown).await;
    watcher.abort();
    app.close().await?;
    result
}
