use crate::config::WorkerConfig;
use crate::connection::R2Client;
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::DatabaseConnection;
use sitemap_rs::sitemap::Sitemap;
use sitemap_rs::sitemap_index::SitemapIndex;
use sitemap_rs::url::Url;
use sitemap_rs::url_set::UrlSet;
use tracing::{error, info};

const SITEMAP_PREFIX: &str = "sitemaps/";

/// Generate and upload sitemap to R2.
///
/// AxumKit core currently publishes only a minimal sitemap (frontend root).
pub async fn generate_and_upload_sitemap(
    _db: &DatabaseConnection,
    r2_client: &R2Client,
    config: &WorkerConfig,
) {
    info!("Starting sitemap generation...");

    match generate_sitemap_internal(r2_client, config).await {
        Ok(()) => {
            info!("Sitemap generation completed successfully");
        }
        Err(e) => {
            error!(error = %e, "Failed to generate sitemap");
        }
    }
}

async fn generate_sitemap_internal(
    r2_client: &R2Client,
    config: &WorkerConfig,
) -> anyhow::Result<()> {
    let base_url = config.frontend_host.trim_end_matches('/');
    let now = Utc::now();
    let now_fixed: DateTime<FixedOffset> = now.into();

    let root_url = Url::builder(base_url.to_string())
        .last_modified(now_fixed)
        .build()?;

    let url_set = UrlSet::new(vec![root_url])?;
    let mut sitemap_buf: Vec<u8> = Vec::new();
    url_set.write(&mut sitemap_buf)?;

    let sitemap_key = format!("{}sitemap-1.xml", SITEMAP_PREFIX);
    r2_client
        .upload_with_content_type(&sitemap_key, sitemap_buf, "application/xml")
        .await?;

    let index = SitemapIndex::new(vec![Sitemap::new(
        r2_client.get_public_url(&sitemap_key),
        Some(now_fixed),
    )])?;
    let mut index_buf: Vec<u8> = Vec::new();
    index.write(&mut index_buf)?;

    let index_key = format!("{}sitemap-index.xml", SITEMAP_PREFIX);
    r2_client
        .upload_with_content_type(&index_key, index_buf, "application/xml")
        .await?;

    info!(
        index_url = r2_client.get_public_url(&index_key),
        "Uploaded sitemap index"
    );

    Ok(())
}
