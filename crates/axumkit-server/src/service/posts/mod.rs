pub mod create_post;
pub mod delete_post;
pub mod get_post;
pub mod list_posts;
pub mod update_post;

pub use create_post::service_create_post;
pub use delete_post::service_delete_post;
pub use get_post::service_get_post;
pub use list_posts::service_list_posts;
pub use update_post::service_update_post;
