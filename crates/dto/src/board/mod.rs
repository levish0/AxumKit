pub mod request;
pub mod response;

pub use request::{
    BoardPostModerationRequest, BoardPostReorderPinsRequest, CreateBoardCommentRequest,
    CreateBoardPostRequest, CreateBoardRequest, DeleteBoardCommentRequest, DeleteBoardPostRequest,
    DeleteBoardRequest, GetBoardBySlugRequest, GetBoardCommentsRequest, GetBoardPermissionsRequest,
    GetBoardPostRequest, GetBoardPostsRequest, GetBoardRequest, GetBoardsRequest,
    ParseBoardRequest, UpdateBoardCommentRequest, UpdateBoardPostRequest, UpdateBoardRequest,
};

pub use response::{
    BoardCommentListResponse, BoardCommentResponse, BoardListResponse, BoardPermissionsResponse,
    BoardPostListResponse, BoardPostModerationResponse, BoardPostReorderPinsResponse,
    BoardPostResponse, BoardResponse, CreateBoardCommentResponse, CreateBoardPostResponse,
    CreateBoardResponse, DeleteBoardCommentResponse, DeleteBoardPostResponse, DeleteBoardResponse,
    UpdateBoardCommentResponse, UpdateBoardPostResponse, UpdateBoardResponse,
};
