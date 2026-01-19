use crate::connection::MeilisearchClient;
use axumkit_dto::search::{SearchUsersRequest, SearchUsersResponse, UserSearchItem};
use axumkit_errors::errors::{Errors, ServiceResult};
use serde::Deserialize;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct IndexedUser {
    id: String,
    handle: String,
    display_name: String,
    bio: Option<String>,
    profile_image: Option<String>,
}

pub async fn service_search_users(
    client: &MeilisearchClient,
    request: &SearchUsersRequest,
) -> ServiceResult<SearchUsersResponse> {
    let query = request.query.as_deref().unwrap_or("");
    info!(
        "Searching users: query='{}', page={}, page_size={}",
        query, request.page, request.page_size
    );

    // Build and execute search query using page/hitsPerPage mode for exact total_hits
    let index = client.get_client().index("users");
    let mut search_query = index.search();

    // Empty query returns all users (MeiliSearch behavior)
    search_query.with_query(query);
    search_query.with_page(request.page as usize);
    search_query.with_hits_per_page(request.page_size as usize);

    let results = search_query.execute::<IndexedUser>().await.map_err(|e| {
        tracing::error!("MeiliSearch user search failed: {}", e);
        Errors::MeiliSearchQueryFailed
    })?;

    // Get pagination info from response (available in page/hitsPerPage mode)
    let total_hits = results.total_hits.unwrap_or(0) as u64;
    let total_pages = results.total_pages.unwrap_or(0) as u32;

    let users: Vec<UserSearchItem> = results
        .hits
        .into_iter()
        .filter_map(|hit| {
            let user = hit.result;
            match Uuid::parse_str(&user.id) {
                Ok(id) => Some(UserSearchItem {
                    id,
                    handle: user.handle,
                    display_name: user.display_name,
                    bio: user.bio,
                    profile_image: user.profile_image,
                }),
                Err(e) => {
                    warn!(
                        "Invalid UUID in user search index: '{}', error: {}",
                        user.id, e
                    );
                    None
                }
            }
        })
        .collect();

    Ok(SearchUsersResponse {
        users,
        page: request.page,
        page_size: request.page_size,
        total_hits,
        total_pages,
    })
}
