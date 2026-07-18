use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetBoardPostsRequest {
    pub board_id: Uuid,
    #[validate(range(min = 1, message = "Page must be greater than 0."))]
    pub page: u32,
    #[validate(range(min = 1, max = 50, message = "Page size must be between 1 and 50."))]
    pub page_size: u32,
}
