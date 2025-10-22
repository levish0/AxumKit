use crate::config::db_config::DbConfig;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{error, info};

// 이 모듈은 데이터베이스 연결을 관리합니다.
// PostgreSQL 데이터베이스에 연결하고 연결 풀을 설정하는 기능을 제공합니다.

/// 데이터베이스 연결을 설정하고 반환하는 함수
///
/// 이 함수는 애플리케이션 시작 시 호출되어 PostgreSQL 데이터베이스에 연결합니다.
/// 설정 파일에서 연결 정보를 읽어와 연결 풀을 구성합니다.
///
/// # 반환값
/// * `DatabaseConnection` - 성공적으로 설정된 데이터베이스 연결 객체
pub async fn establish_connection() -> DatabaseConnection {
    // 환경 설정에서 데이터베이스 연결 정보를 가져와 URL 생성
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        &DbConfig::get().db_user,
        &DbConfig::get().db_password,
        &DbConfig::get().db_host,
        &DbConfig::get().db_port,
        &DbConfig::get().db_name
    );
    info!("Attempting to connect to connection: {}", database_url);

    // 연결 옵션 설정
    let mut options = ConnectOptions::new(database_url);
    options
        // 연결 풀 크기 설정
        .max_connections(DbConfig::get().db_max_connection) // 최대 연결 수
        .min_connections(DbConfig::get().db_min_connection) // 최소 연결 수
        // 타임아웃 설정
        .connect_timeout(Duration::from_secs(8)) // 연결 타임아웃: 8초
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300)) // 유휴 타임아웃: 5분
        // SQL 로깅 활성화 (디버깅 용도)
        .sqlx_logging(false);

    // 데이터베이스 연결 시도 및 결과 처리
    match Database::connect(options).await {
        Ok(connection) => {
            // 연결 성공 시 로그 출력 후 연결 객체 반환
            info!("Successfully connected to the connection.");
            connection
        }
        Err(err) => {
            // 연결 실패 시 에러 로그 출력 후 애플리케이션 종료
            // 데이터베이스 연결은 애플리케이션의 핵심 요구사항이므로 실패 시 계속 진행할 수 없음
            error!("Failed to connect to connection: {}", err);
            panic!("Failed to connect to connection");
        }
    }
}
