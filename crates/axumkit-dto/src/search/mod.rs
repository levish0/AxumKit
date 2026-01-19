pub mod request;
pub mod response;

pub use request::{SearchPostsRequest, SearchUsersRequest, SortOrder};
pub use response::{PostSearchHit, SearchPostsResponse, SearchUsersResponse, UserSearchItem};
