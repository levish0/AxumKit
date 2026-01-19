use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub enum SortOrder {
    Asc,
    Desc,
}
