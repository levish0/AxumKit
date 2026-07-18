mod create;
mod delete;
mod list;
mod update;

pub use create::service_create_board_comment;
pub use delete::service_delete_board_comment;
pub use list::service_list_board_comments;
pub use update::service_update_board_comment;
