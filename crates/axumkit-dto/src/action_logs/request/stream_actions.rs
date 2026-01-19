use axumkit_constants::ActionLogAction;
use axumkit_entity::common::ActionResourceType;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct StreamActionsQuery {
    /// Filter by user ID (for contributions)
    pub user_id: Option<Uuid>,
    /// Filter by resource ID
    pub resource_id: Option<Uuid>,
    /// Filter by resource type
    pub resource_type: Option<ActionResourceType>,
    /// Filter by actions
    pub actions: Option<Vec<ActionLogAction>>,
}
