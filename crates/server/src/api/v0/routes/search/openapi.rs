use dto::search::{SearchUsersRequest, SearchUsersResponse, SortOrder, UserSearchItem};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::search_users::search_users,
    ),
    components(
        schemas(
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
