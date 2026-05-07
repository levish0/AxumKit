//! AxumKit Error Types

pub mod errors;
pub mod handlers;
pub mod protocol;

pub use errors::{ErrorResponse, Errors, ServiceResult, handler_404};
