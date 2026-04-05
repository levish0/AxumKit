use axumkit_config::ServerConfig;

/// Convert an R2 storage key to a full public URL
pub fn build_r2_public_url(key: &str) -> String {
    let config = ServerConfig::get();
    format!("{}/{}", config.r2_assets_public_domain, key)
}
