use axumkit_dto::posts::{
    CreatePostRequest, CreatePostResponse, DeletePostResponse, GetPostPath, ListPostsQuery,
    ListPostsResponse, PostListItem, PostResponse, UpdatePostRequest,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::create_post::create_post,
        super::get_post::get_post,
        super::list_posts::list_posts,
        super::update_post::update_post,
        super::delete_post::delete_post,
    ),
    components(
        schemas(
            CreatePostRequest,
            CreatePostResponse,
            GetPostPath,
            ListPostsQuery,
            ListPostsResponse,
            PostListItem,
            PostResponse,
            UpdatePostRequest,
            DeletePostResponse,
        )
    ),
    tags(
        (name = "Posts", description = "Posts endpoints")
    )
)]
pub struct PostsApiDoc;
