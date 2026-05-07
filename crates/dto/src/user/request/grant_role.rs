use crate::validator::datetime_validator::validate_future_datetime;
use crate::validator::string_validator::validate_not_blank;
use chrono::{DateTime, Utc};
use entity::common::Role;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for grant role request.
pub struct GrantRoleRequest {
    pub user_id: Uuid,
    pub role: Role,
    /// Role expiration time (None = permanent)
    #[validate(custom(function = "validate_future_datetime"))]
    pub expires_at: Option<DateTime<Utc>>,
    #[validate(length(
        min = 1,
        max = 1000,
        message = "Reason must be between 1 and 1000 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_grant_role_request_rejects_past_expires_at() {
        let req = GrantRoleRequest {
            user_id: Uuid::now_v7(),
            role: Role::Mod,
            expires_at: Some(Utc::now() - Duration::minutes(1)),
            reason: "test".to_string(),
        };

        let err = req
            .validate()
            .expect_err("past expires_at must be rejected");
        let field_errors = err.field_errors();
        let field = field_errors
            .get("expires_at")
            .expect("expires_at field error must exist");
        assert!(
            field
                .iter()
                .any(|e| e.code.as_ref() == "expires_at_must_be_in_future"),
            "expected expires_at_must_be_in_future error"
        );
    }

    #[test]
    fn test_grant_role_request_accepts_explicit_role() {
        let req = GrantRoleRequest {
            user_id: Uuid::now_v7(),
            role: Role::Mod,
            expires_at: Some(Utc::now() + Duration::minutes(10)),
            reason: "test".to_string(),
        };

        assert!(req.validate().is_ok());
    }
}
