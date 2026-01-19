use crate::config::WorkerConfig;
use crate::connection::R2Client;
use axumkit_entity::posts;
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::prelude::DateTimeUtc;
use sea_orm::{ColumnTrait, DatabaseConnection, FromQueryResult, QueryFilter, QueryOrder};
use sitemap_rs::sitemap::Sitemap;
use sitemap_rs::sitemap_index::SitemapIndex;
use sitemap_rs::url::Url;
use sitemap_rs::url_set::UrlSet;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Sitemap post projection - only needed columns
#[derive(Debug, FromQueryResult)]
struct SitemapPost {
    id: Uuid,
    updated_at: DateTimeUtc,
}

const MAX_URLS_PER_SITEMAP: usize = 50_000;
const SITEMAP_PREFIX: &str = "sitemaps/";
const BATCH_SIZE: u64 = 10_000;

/// Generate and upload sitemap to R2
pub async fn generate_and_upload_sitemap(
    db: &DatabaseConnection,
    r2_client: &R2Client,
    config: &WorkerConfig,
) {
    info!("Starting sitemap generation...");

    match generate_sitemap_internal(db, r2_client, config).await {
        Ok(count) => {
            info!(
                post_count = count,
                "Sitemap generation completed successfully"
            );
        }
        Err(e) => {
            error!(error = %e, "Failed to generate sitemap");
        }
    }
}

async fn generate_sitemap_internal(
    db: &DatabaseConnection,
    r2_client: &R2Client,
    config: &WorkerConfig,
) -> anyhow::Result<usize> {
    let base_url = &config.frontend_host;
    let now = Utc::now();
    let now_fixed: DateTime<FixedOffset> = now.into();

    let mut urls: Vec<Url> = Vec::with_capacity(MAX_URLS_PER_SITEMAP);
    let mut sitemap_entries: Vec<Sitemap> = Vec::new();
    let mut sitemap_num = 0;
    let mut total_count = 0;
    let mut after_id: Option<Uuid> = None;

    loop {
        // Fetch batch using cursor
        let post_list = fetch_posts_batch(db, after_id, BATCH_SIZE).await?;

        if post_list.is_empty() {
            break;
        }

        // Update cursor for next batch
        after_id = post_list.last().map(|p| p.id);

        for post in post_list {
            let url_string = build_post_url(base_url, post.id);
            match Url::builder(url_string.clone())
                .last_modified(post.updated_at.into())
                .build()
            {
                Ok(url) => urls.push(url),
                Err(e) => {
                    warn!(url = %url_string, error = %e, "Failed to build sitemap URL");
                }
            }

            total_count += 1;

            // Upload sitemap when reaching limit
            if urls.len() >= MAX_URLS_PER_SITEMAP {
                sitemap_num += 1;
                upload_sitemap_chunk(
                    r2_client,
                    &mut urls,
                    sitemap_num,
                    now_fixed,
                    &mut sitemap_entries,
                )
                .await?;
            }
        }
    }

    // Upload remaining URLs
    if !urls.is_empty() {
        sitemap_num += 1;
        upload_sitemap_chunk(
            r2_client,
            &mut urls,
            sitemap_num,
            now_fixed,
            &mut sitemap_entries,
        )
        .await?;
    }

    // Create sitemap index (even if empty)
    let sitemap_index = SitemapIndex::new(sitemap_entries)?;
    let mut index_buf: Vec<u8> = Vec::new();
    sitemap_index.write(&mut index_buf)?;

    let index_key = format!("{}sitemap-index.xml", SITEMAP_PREFIX);
    r2_client
        .upload_with_content_type(&index_key, index_buf, "application/xml")
        .await?;

    info!(
        index_url = r2_client.get_r2_public_url(&index_key),
        sitemap_count = sitemap_num,
        "Uploaded sitemap index"
    );

    // Cleanup old sitemap files
    cleanup_old_sitemaps(r2_client, sitemap_num).await;

    Ok(total_count)
}

/// Fetch posts batch using UUID cursor pagination (select only needed columns)
async fn fetch_posts_batch(
    db: &DatabaseConnection,
    after_id: Option<Uuid>,
    batch_size: u64,
) -> anyhow::Result<Vec<SitemapPost>> {
    use sea_orm::{EntityTrait, QuerySelect};

    let mut query = posts::Entity::find()
        .select_only()
        .column(posts::Column::Id)
        .column(posts::Column::UpdatedAt)
        .order_by_asc(posts::Column::Id);

    if let Some(cursor) = after_id {
        query = query.filter(posts::Column::Id.gt(cursor));
    }

    let post_list = query
        .limit(batch_size)
        .into_model::<SitemapPost>()
        .all(db)
        .await?;
    Ok(post_list)
}

async fn upload_sitemap_chunk(
    r2_client: &R2Client,
    urls: &mut Vec<Url>,
    sitemap_num: usize,
    lastmod: DateTime<FixedOffset>,
    sitemap_entries: &mut Vec<Sitemap>,
) -> anyhow::Result<()> {
    let sitemap_key = format!("{}sitemap-{}.xml", SITEMAP_PREFIX, sitemap_num);
    let chunk_urls = std::mem::take(urls);
    let url_count = chunk_urls.len();

    let url_set = UrlSet::new(chunk_urls)?;
    let mut buf: Vec<u8> = Vec::new();
    url_set.write(&mut buf)?;

    r2_client
        .upload_with_content_type(&sitemap_key, buf, "application/xml")
        .await?;

    info!(sitemap_num, url_count, "Uploaded sitemap file");

    let sitemap_url = r2_client.get_r2_public_url(&sitemap_key);
    sitemap_entries.push(Sitemap::new(sitemap_url, Some(lastmod)));

    Ok(())
}

/// Delete old sitemap files beyond current count
async fn cleanup_old_sitemaps(r2_client: &R2Client, current_count: usize) {
    // List all files in sitemaps/ folder
    let keys = match r2_client.list_objects(SITEMAP_PREFIX).await {
        Ok(keys) => keys,
        Err(e) => {
            warn!(error = %e, "Failed to list sitemap files for cleanup");
            return;
        }
    };

    // Delete sitemap-{N}.xml where N > current_count
    for key in keys {
        // Parse sitemap number from key (e.g., "sitemaps/sitemap-5.xml" -> 5)
        if let Some(num_str) = key
            .strip_prefix(SITEMAP_PREFIX)
            .and_then(|s| s.strip_prefix("sitemap-"))
            .and_then(|s| s.strip_suffix(".xml"))
        {
            if let Ok(num) = num_str.parse::<usize>() {
                if num > current_count {
                    if let Err(e) = r2_client.delete(&key).await {
                        warn!(key, error = %e, "Failed to delete old sitemap file");
                    } else {
                        info!(key, "Deleted old sitemap file");
                    }
                }
            }
        }
    }
}

fn build_post_url(base_url: &str, post_id: Uuid) -> String {
    format!("{}/posts/{}", base_url, post_id)
}
