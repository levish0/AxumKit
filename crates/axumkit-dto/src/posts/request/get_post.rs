use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
#[into_params(parameter_in = Path)]
pub struct GetPostPath {
    pub id: Uuid,
}
