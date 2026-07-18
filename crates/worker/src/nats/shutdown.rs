//! Process-wide graceful-shutdown signal.
//!
//! On SIGTERM/SIGINT the worker should stop pulling new messages and let
//! in-flight handlers finish before the process exits, so a deploy does not drop
//! jobs mid-side-effect. Shutdown is genuinely process-wide, so it lives in a
//! single global watch channel that every consumer's run loop selects on and
//! `main` triggers — no per-consumer wiring.

use std::sync::OnceLock;
use tokio::sync::watch;

static SHUTDOWN: OnceLock<watch::Sender<bool>> = OnceLock::new();

fn sender() -> &'static watch::Sender<bool> {
    SHUTDOWN.get_or_init(|| watch::channel(false).0)
}

/// Signals every consumer to stop pulling and drain. Idempotent.
pub fn trigger() {
    let _ = sender().send(true);
}

/// Resolves once shutdown has been triggered (immediately if already triggered).
pub async fn wait() {
    let mut rx = sender().subscribe();
    if *rx.borrow() {
        return;
    }
    let _ = rx.changed().await;
}
