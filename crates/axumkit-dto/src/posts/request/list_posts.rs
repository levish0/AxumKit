use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListPostsQuery {
    pub limit: u64,
    pub offset: u64,
}
