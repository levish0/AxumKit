use crate::http::validation;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Deserialize, IntoParams, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct PaginationQuery {
    #[validate(range(min = 1, max = 100))]
    #[param(minimum = 1, maximum = 100)]
    pub limit: Option<u64>,
    #[validate(length(max = 512))]
    #[param(max_length = 512)]
    pub cursor: Option<String>,
}
#[derive(Deserialize, IntoParams, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct SearchQuery {
    #[validate(custom(function = "validation::search_query"))]
    pub q: String,
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<usize>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<usize>,
}
#[derive(Deserialize, IntoParams, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct VersionQuery {
    #[validate(range(min = 1))]
    pub expected_version: i64,
}
