use std::sync::LazyLock;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, fmt};

static TRACING_GUARD: LazyLock<Option<tracing_appender::non_blocking::WorkerGuard>> =
    LazyLock::new(|| {
        // EnvFilter: RUST_LOG 환경변수로 제어 가능
        // 기본값: debug 빌드는 debug, release 빌드는 info
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            let default = "debug";

            #[cfg(not(debug_assertions))]
            let default = "info";

            EnvFilter::new(default)
        });

        #[cfg(debug_assertions)]
        {
            // 개발 환경: 콘솔 + 파일 (human-readable)
            std::fs::create_dir_all("logs").expect("Failed to create logs directory");

            let file_appender = tracing_appender::rolling::daily("logs", "app.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .with_writer(std::io::stdout)
                        .with_filter(env_filter.clone()),
                )
                .with(
                    fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(false)
                        .with_filter(env_filter),
                )
                .init();

            info!("Tracing initialized (development mode: console + file)");
            Some(guard)
        }

        #[cfg(not(debug_assertions))]
        {
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .json() // 로그 수집 시스템(ELK, Loki, Datadog)이 파싱 가능
                        .with_writer(std::io::stdout)
                        .with_filter(env_filter),
                )
                .init();

            info!("Tracing initialized (production mode: JSON stdout)");
            None // 파일 appender 없으므로 guard 필요 없음
        }
    });

pub fn init_tracing() {
    // LazyLock을 강제로 초기화
    LazyLock::force(&TRACING_GUARD);
}
