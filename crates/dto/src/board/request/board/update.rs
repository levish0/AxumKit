use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateBoardRequest {
    pub board_id: Uuid,
    #[validate(length(
        min = 1,
        max = 128,
        message = "Slug must be between 1 and 128 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub slug: Option<String>,
    #[validate(length(
        min = 1,
        max = 256,
        message = "Name must be between 1 and 256 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub name: Option<String>,
    #[serde(default, with = "serde_with::rust::double_option")]
    #[schema(nullable = true, max_length = 500)]
    #[validate(length(max = 500, message = "Description cannot exceed 500 characters."))]
    pub description: Option<Option<String>>,
    pub order: Option<i32>,
    pub is_disabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::UpdateBoardRequest;

    #[test]
    fn distinguishes_missing_null_and_string_description() {
        let missing: UpdateBoardRequest =
            serde_json::from_str(r#"{"board_id":"018f06d7-8b5d-7cc8-a93f-30f6f37cf7e0"}"#).unwrap();
        assert_eq!(missing.description, None);

        let null: UpdateBoardRequest = serde_json::from_str(
            r#"{"board_id":"018f06d7-8b5d-7cc8-a93f-30f6f37cf7e0","description":null}"#,
        )
        .unwrap();
        assert_eq!(null.description, Some(None));

        let value: UpdateBoardRequest = serde_json::from_str(
            r#"{"board_id":"018f06d7-8b5d-7cc8-a93f-30f6f37cf7e0","description":"notice board"}"#,
        )
        .unwrap();
        assert_eq!(value.description, Some(Some("notice board".to_string())));
    }
}
