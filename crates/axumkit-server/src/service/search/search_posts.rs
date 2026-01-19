use crate::connection::MeilisearchClient;
use axumkit_dto::search::{PostSearchHit, SearchPostsRequest, SearchPostsResponse};
use axumkit_errors::errors::{Errors, ServiceResult};
use serde::Deserialize;
use tracing::{info, warn};
use uuid::Uuid;

// Private: MeiliSearch index schema
#[derive(Debug, Deserialize)]
struct IndexedPost {
    id: String,
    author_id: String,
    title: String,
    content: String,
}

pub async fn service_search_posts(
    client: &MeilisearchClient,
    request: &SearchPostsRequest,
) -> ServiceResult<SearchPostsResponse> {
    info!(
        "Searching posts: query='{}', page={}, page_size={}",
        request.query, request.page, request.page_size
    );

    // Build and execute search query using page/hitsPerPage mode for exact total_hits
    let index = client.get_client().index("posts");
    let mut search_query = index.search();

    search_query.with_query(&request.query);
    search_query.with_page(request.page as usize);
    search_query.with_hits_per_page(request.page_size as usize);

    // Execute search
    let results = search_query.execute::<IndexedPost>().await.map_err(|e| {
        tracing::error!("MeiliSearch query failed: {}", e);
        Errors::MeiliSearchQueryFailed
    })?;

    // Get pagination info from response (available in page/hitsPerPage mode)
    let total_hits = results.total_hits.unwrap_or(0) as u64;
    let total_pages = results.total_pages.unwrap_or(0) as u32;

    // Transform results to DTOs
    let hits: Vec<PostSearchHit> = results
        .hits
        .into_iter()
        .filter_map(|hit| {
            let post = hit.result;
            let id = match Uuid::parse_str(&post.id) {
                Ok(id) => id,
                Err(e) => {
                    warn!("Invalid UUID in search index: '{}', error: {}", post.id, e);
                    return None;
                }
            };
            let author_id = match Uuid::parse_str(&post.author_id) {
                Ok(id) => id,
                Err(e) => {
                    warn!(
                        "Invalid author_id UUID in search index: '{}', error: {}",
                        post.author_id, e
                    );
                    return None;
                }
            };
            Some(PostSearchHit {
                id,
                author_id,
                title: post.title,
                content_snippet: truncate_content(&post.content, 200),
            })
        })
        .collect();

    Ok(SearchPostsResponse {
        hits,
        page: request.page,
        page_size: request.page_size,
        total_hits,
        total_pages,
    })
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    match content.char_indices().nth(max_chars) {
        Some((idx, _)) => content[..idx].to_string(),
        None => content.to_string(),
    }
}
