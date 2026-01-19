use axumkit_dto::search::{
    PostSearchHit, SearchPostsRequest, SearchPostsResponse, SearchUsersRequest,
    SearchUsersResponse, SortOrder, UserSearchItem,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::search_posts::search_posts,
        super::search_users::search_users,
    ),
    components(
        schemas(
            SearchPostsRequest,
            SearchPostsResponse,
            PostSearchHit,
            SortOrder,
            SearchUsersRequest,
            SearchUsersResponse,
            UserSearchItem,
        )
    ),
    tags(
        (name = "Search", description = "Search endpoints")
    )
)]
pub struct SearchApiDoc;
