use crate::http::validation;
use application::posts::{CommentSummary, PostSummary, SavePost};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PostStateInput {
    Draft,
    Published,
}
impl From<PostStateInput> for application::posts::PostPublication {
    fn from(value: PostStateInput) -> Self {
        match value {
            PostStateInput::Draft => Self::Draft,
            PostStateInput::Published => Self::Published,
        }
    }
}

#[derive(Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PostInput {
    #[validate(custom(function = "validation::post_title"))]
    #[schema(min_length = 1, max_length = 200)]
    pub title: String,
    #[validate(custom(function = "validation::post_body"))]
    #[schema(min_length = 1, max_length = 8000)]
    pub body: String,
    pub state: PostStateInput,
    #[validate(custom(function = "validation::post_attachments"))]
    #[schema(max_items = 10)]
    pub attachment_ids: Vec<Uuid>,
    /// Zero when creating; the current version when updating.
    #[validate(range(min = 0))]
    pub expected_version: i64,
}
impl From<PostInput> for SavePost {
    fn from(value: PostInput) -> Self {
        Self {
            title: value.title,
            body: value.body,
            state: value.state.into(),
            attachment_ids: value.attachment_ids,
            expected_version: value.expected_version,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PostOutput {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub body: String,
    pub state: PostStateOutput,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attachment_ids: Vec<Uuid>,
}
impl From<PostSummary> for PostOutput {
    fn from(row: PostSummary) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            author_id: row.author_id,
            title: row.title,
            body: row.body,
            state: row.state.into(),
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
            attachment_ids: row.attachment_ids,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PostPageOutput {
    pub items: Vec<PostOutput>,
    pub next_cursor: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CommentOutput {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub version: i64,
    pub created_at: DateTime<Utc>,
}
impl From<CommentSummary> for CommentOutput {
    fn from(row: CommentSummary) -> Self {
        Self {
            id: row.id,
            post_id: row.post_id,
            author_id: row.author_id,
            body: row.body,
            version: row.version,
            created_at: row.created_at,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PostStateOutput {
    Draft,
    Published,
    Deleted,
}
impl From<application::posts::PostState> for PostStateOutput {
    fn from(value: application::posts::PostState) -> Self {
        match value {
            application::posts::PostState::Draft => Self::Draft,
            application::posts::PostState::Published => Self::Published,
            application::posts::PostState::Deleted => Self::Deleted,
        }
    }
}
impl From<PostStateOutput> for application::posts::PostState {
    fn from(value: PostStateOutput) -> Self {
        match value {
            PostStateOutput::Draft => Self::Draft,
            PostStateOutput::Published => Self::Published,
            PostStateOutput::Deleted => Self::Deleted,
        }
    }
}
