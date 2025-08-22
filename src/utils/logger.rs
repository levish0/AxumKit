use std::sync::LazyLock;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, fmt};

static TRACING_GUARD: LazyLock<tracing_appender::non_blocking::WorkerGuard> = LazyLock::new(|| {
    // logs 디렉토리가 없으면 생성
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");

    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    #[cfg(debug_assertions)]
    {
        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_writer(std::io::stdout)
                    .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
            )
            .with(
                fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
            )
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
            )
            .init();
    }

    info!("Tracing initialized successfully");

    guard
});

pub fn init_tracing() {
    // LazyLock을 강제로 초기화
    LazyLock::force(&TRACING_GUARD);
}