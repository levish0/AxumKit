pub mod create;
pub mod delete;
pub mod get;
pub mod get_by_slug;
pub mod get_list;
pub mod update;

pub use create::CreateBoardRequest;
pub use delete::DeleteBoardRequest;
pub use get::GetBoardRequest;
pub use get_by_slug::GetBoardBySlugRequest;
pub use get_list::GetBoardsRequest;
pub use update::UpdateBoardRequest;
