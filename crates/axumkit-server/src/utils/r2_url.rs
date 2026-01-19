use axumkit_config::ServerConfig;

/// R2 스토리지 key를 전체 public URL로 변환
pub fn build_r2_public_url(key: &str) -> String {
    let config = ServerConfig::get();
    format!("{}/{}", config.r2_public_domain, key)
}
