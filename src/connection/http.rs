use reqwest::Client;
use std::time::Duration;
use tracing::{error, info};

pub async fn create_http_client() -> Result<Client, reqwest::Error> {
    info!("Creating HTTP client");

    let client = Client::builder()
        .timeout(Duration::from_secs(30)) // 전체 요청 타임아웃
        .connect_timeout(Duration::from_secs(10)) // 연결 타임아웃
        .pool_idle_timeout(Duration::from_secs(90)) // 유휴 연결 타임아웃
        .pool_max_idle_per_host(10) // 호스트당 최대 유휴 연결 수
        .user_agent("mofumofu/1.0") // User-Agent 설정
        .tcp_keepalive(Duration::from_secs(60)) // TCP keep-alive
        .build()
        .map_err(|e| {
            error!("Failed to create HTTP client: {:?}", e);
            e
        })?;

    info!("Successfully created HTTP client");
    Ok(client)
}
