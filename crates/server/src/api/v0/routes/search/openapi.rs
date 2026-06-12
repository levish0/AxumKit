use dto::search::{SearchUsersRequest, SearchUsersResponse, SortOrder, UserSearchItem};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::users::search_users,
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
