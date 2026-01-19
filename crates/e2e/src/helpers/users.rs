//! User creation helpers for E2E tests

use anyhow::Result;
use axumkit_dto::auth::request::LoginRequest;
use axumkit_dto::user::request::CreateUserRequest;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::fixtures::{ApiClient, TestServer};
use axumkit_entity::users;

/// Test user with credentials and session
#[derive(Debug, Clone)]
pub struct TestUser {
    pub id: Uuid,
    pub handle: String,
    pub email: String,
    pub password: String,
    pub session_id: String,
}

impl TestUser {
    /// Get an authenticated API client for this user
    pub fn client(&self, server: &TestServer) -> ApiClient {
        server.authenticated_client(&self.session_id)
    }
}

impl TestServer {
    /// Create a regular verified user
    pub async fn create_user(&self) -> Result<TestUser> {
        let unique_id = &Uuid::new_v4().to_string()[..8];
        let handle = format!("test{}", unique_id);
        let email = format!("{}@test.local", handle);
        let password = "testpass123".to_string();

        // 1. Create user via API
        let create_req = CreateUserRequest {
            email: email.clone(),
            handle: handle.clone(),
            display_name: handle.clone(),
            password: password.clone(),
        };

        let response = self.client.post("/v0/users", &create_req).await?;
        if response.status().as_u16() != 201 {
            let body = response.text().await?;
            return Err(anyhow::anyhow!("Failed to create user: {}", body));
        }

        // 2. Get user from DB and verify email
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(&email))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found after creation"))?;

        let user_id = user.id;

        // 3. Set verified_at via SeaORM
        let mut active: users::ActiveModel = user.into();
        active.verified_at = Set(Some(chrono::Utc::now().into()));
        active.update(&self.db).await?;

        // 4. Login and get session
        let login_req = LoginRequest {
            email: email.clone(),
            password: password.clone(),
            remember_me: false,
        };

        let response = self.client.post("/v0/auth/login", &login_req).await?;
        if response.status().as_u16() != 204 {
            let body = response.text().await?;
            return Err(anyhow::anyhow!("Failed to login: {}", body));
        }

        // 5. Extract session_id from Set-Cookie header
        let session_id = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .find_map(|v| {
                let cookie_str = v.to_str().ok()?;
                if cookie_str.starts_with("session_id=") {
                    let end = cookie_str.find(';').unwrap_or(cookie_str.len());
                    Some(cookie_str[11..end].to_string())
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("session_id cookie not found in response"))?;

        Ok(TestUser {
            id: user_id,
            handle,
            email,
            password,
            session_id,
        })
    }
}
