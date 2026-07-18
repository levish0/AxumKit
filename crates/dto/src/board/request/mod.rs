pub mod board;
pub mod comment;
pub mod parse;
pub mod permissions;
pub mod post;

pub use board::{
    CreateBoardRequest, DeleteBoardRequest, GetBoardBySlugRequest, GetBoardRequest,
    GetBoardsRequest, UpdateBoardRequest,
};
pub use comment::{
    CreateBoardCommentRequest, DeleteBoardCommentRequest, GetBoardCommentsRequest,
    UpdateBoardCommentRequest,
};
pub use parse::ParseBoardRequest;
pub use permissions::GetBoardPermissionsRequest;
pub use post::{
    BoardPostModerationRequest, BoardPostReorderPinsRequest, CreateBoardPostRequest,
    DeleteBoardPostRequest, GetBoardPostRequest, GetBoardPostsRequest, UpdateBoardPostRequest,
};
