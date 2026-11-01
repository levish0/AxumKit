use axum::Router;
use std::{future::IntoFuture, time::Duration};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

pub async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let server = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown.clone().cancelled_owned())
    .into_future();
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => result?,
        _ = shutdown.cancelled() => {
            match tokio::time::timeout(Duration::from_secs(30), &mut server).await {
                Ok(result) => result?,
                Err(_) => tracing::warn!("HTTP drain deadline reached"),
            }
        }
    }
    Ok(())
}

pub async fn shutdown_signal() -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let mut termination =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! { result = tokio::signal::ctrl_c() => result?, _ = termination.recv() => {} }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await?;
    Ok(())
}
