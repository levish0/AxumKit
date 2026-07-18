pub mod create;
pub mod delete;
pub mod get;
pub mod get_list;
pub mod update;

pub use create::CreateBoardCommentResponse;
pub use delete::DeleteBoardCommentResponse;
pub use get::BoardCommentResponse;
pub use get_list::BoardCommentListResponse;
pub use update::UpdateBoardCommentResponse;
