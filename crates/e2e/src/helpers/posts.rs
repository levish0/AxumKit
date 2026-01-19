//! Post creation/editing helpers for E2E tests

use anyhow::Result;
use axumkit_dto::posts::{CreatePostRequest, PostResponse, UpdatePostRequest};
use uuid::Uuid;

use crate::fixtures::TestServer;
use crate::helpers::TestUser;

/// Test post with current state
#[derive(Debug, Clone)]
pub struct TestPost {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub content: String,
}

impl TestServer {
    /// Create a post as the given user
    pub async fn create_post(
        &self,
        user: &TestUser,
        title: &str,
        content: &str,
    ) -> Result<TestPost> {
        let client = user.client(self);

        let req = CreatePostRequest {
            title: title.to_string(),
            content: content.to_string(),
        };

        let response = client.post("/v0/posts", &req).await?;
        let status = response.status().as_u16();

        if status != 200 && status != 201 {
            let body = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to create post ({}): {}",
                status,
                body
            ));
        }

        // CreatePostResponse only returns id, so we need to get full post
        #[derive(serde::Deserialize)]
        struct CreateResponse {
            id: String,
        }
        let create_resp: CreateResponse = response.json().await?;
        let post_id: Uuid = create_resp.id.parse()?;

        // Get full post
        self.get_post(user, post_id).await
    }

    /// Get post by ID
    pub async fn get_post(&self, user: &TestUser, post_id: Uuid) -> Result<TestPost> {
        let client = user.client(self);
        let response = client.get(&format!("/v0/posts/{}", post_id)).await?;
        let status = response.status().as_u16();

        if status != 200 {
            let body = response.text().await?;
            return Err(anyhow::anyhow!("Failed to get post ({}): {}", status, body));
        }

        let post: PostResponse = response.json().await?;

        Ok(TestPost {
            id: post.id.parse()?,
            author_id: post.author_id.parse()?,
            title: post.title,
            content: post.content,
        })
    }

    /// Update a post
    pub async fn update_post(
        &self,
        user: &TestUser,
        post_id: Uuid,
        title: Option<&str>,
        content: Option<&str>,
    ) -> Result<TestPost> {
        let client = user.client(self);

        let req = UpdatePostRequest {
            title: title.map(String::from),
            content: content.map(String::from),
        };

        let response = client
            .patch(&format!("/v0/posts/{}", post_id), &req)
            .await?;
        let status = response.status().as_u16();

        if status != 200 {
            let body = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to update post ({}): {}",
                status,
                body
            ));
        }

        let post: PostResponse = response.json().await?;

        Ok(TestPost {
            id: post.id.parse()?,
            author_id: post.author_id.parse()?,
            title: post.title,
            content: post.content,
        })
    }

    /// Delete a post
    pub async fn delete_post(&self, user: &TestUser, post_id: Uuid) -> Result<()> {
        let client = user.client(self);
        let response = client.delete(&format!("/v0/posts/{}", post_id)).await?;
        let status = response.status().as_u16();

        if status != 200 {
            let body = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to delete post ({}): {}",
                status,
                body
            ));
        }

        Ok(())
    }
}
