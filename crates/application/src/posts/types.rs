use super::entities::{comment, post};
use crate::{authorization::PostFacts, posts::PostState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostPublication {
    Draft,
    #[default]
    Published,
}
impl From<PostPublication> for PostState {
    fn from(value: PostPublication) -> Self {
        match value {
            PostPublication::Draft => Self::Draft,
            PostPublication::Published => Self::Published,
        }
    }
}

impl PostPublication {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
        }
    }
}

pub struct SavePost {
    pub title: String,
    pub body: String,
    pub state: PostPublication,
    pub attachment_ids: Vec<Uuid>,
    pub expected_version: i64,
}

pub struct PostSummary {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub body: String,
    pub state: PostState,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attachment_ids: Vec<Uuid>,
}
impl PostSummary {
    pub(super) fn from_model(row: post::Model, attachment_ids: Vec<Uuid>) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            author_id: row.author_id,
            title: row.title,
            body: row.body,
            state: row.state,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
            attachment_ids,
        }
    }
}
pub struct CommentSummary {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub version: i64,
    pub created_at: DateTime<Utc>,
}
impl From<comment::Model> for CommentSummary {
    fn from(row: comment::Model) -> Self {
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
pub(super) fn facts(post: &post::Model, organization_owner_id: Uuid) -> PostFacts {
    PostFacts {
        id: post.id,
        organization_id: post.organization_id,
        organization_owner_id,
        author_id: post.author_id,
        published: post.state == PostState::Published,
        deleted: post.state == PostState::Deleted,
        organization_member: true,
    }
}
