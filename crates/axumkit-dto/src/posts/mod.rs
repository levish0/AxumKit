pub mod request;
pub mod response;

pub use request::{CreatePostRequest, GetPostPath, ListPostsQuery, UpdatePostRequest};
pub use response::{
    CreatePostResponse, DeletePostResponse, ListPostsResponse, PostListItem, PostResponse,
};
