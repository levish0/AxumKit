use std::sync::LazyLock;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, fmt};

static TRACING_GUARD: LazyLock<Option<tracing_appender::non_blocking::WorkerGuard>> =
    LazyLock::new(|| {
        // EnvFilter: controllable via RUST_LOG environment variable
        // Default: debug for debug builds, info for release builds
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            let default = "debug";

            #[cfg(not(debug_assertions))]
            let default = "info";

            EnvFilter::new(default)
        });

        #[cfg(debug_assertions)]
        {
            // Development: console only (human-readable)
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .with_writer(std::io::stdout)
                        .with_filter(env_filter),
                )
                .init();

            info!("Tracing initialized (development mode: console only)");
            None
        }

        #[cfg(not(debug_assertions))]
        {
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .json() // Parseable by log collection systems (ELK, Loki, Datadog)
                        .with_writer(std::io::stdout)
                        .with_filter(env_filter),
                )
                .init();

            info!("Tracing initialized (production mode: JSON stdout)");
            None
        }
    });

pub fn init_tracing() {
    // Force-initialize LazyLock
    LazyLock::force(&TRACING_GUARD);
}
