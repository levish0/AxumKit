//! HTTP API client for E2E tests

use anyhow::Result;
use reqwest::{Client, Response, StatusCode};
use serde::{Serialize, de::DeserializeOwned};

/// API client for making requests to the server
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    session_cookie: Option<String>,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .cookie_store(true)
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.to_string(),
            session_cookie: None,
        }
    }

    /// Create an authenticated client with a session cookie
    pub fn with_session(base_url: &str, session_cookie: &str) -> Self {
        Self {
            client: Client::builder()
                .cookie_store(true)
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.to_string(),
            session_cookie: Some(session_cookie.to_string()),
        }
    }

    /// Set session cookie
    pub fn set_session(&mut self, session_cookie: &str) {
        self.session_cookie = Some(session_cookie.to_string());
    }

    /// GET request
    pub async fn get(&self, path: &str) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.get(&url);

        if let Some(ref cookie) = self.session_cookie {
            req = req.header("Cookie", format!("session_id={}", cookie));
        }

        Ok(req.send().await?)
    }

    /// GET request with JSON response
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.get(path).await?;
        Ok(resp.json().await?)
    }

    /// POST request with JSON body
    pub async fn post<T: Serialize>(&self, path: &str, body: &T) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);

        if let Some(ref cookie) = self.session_cookie {
            req = req.header("Cookie", format!("session_id={}", cookie));
        }

        Ok(req.send().await?)
    }

    /// POST request with JSON body and additional header
    pub async fn post_with_header<T: Serialize>(
        &self,
        path: &str,
        body: &T,
        header_name: &str,
        header_value: &str,
    ) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self
            .client
            .post(&url)
            .json(body)
            .header(header_name, header_value);

        if let Some(ref cookie) = self.session_cookie {
            req = req.header("Cookie", format!("session_id={}", cookie));
        }

        Ok(req.send().await?)
    }

    /// POST request with JSON body and JSON response
    pub async fn post_json<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let resp = self.post(path, body).await?;
        Ok(resp.json().await?)
    }

    /// PUT request with JSON body
    pub async fn put<T: Serialize>(&self, path: &str, body: &T) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.put(&url).json(body);

        if let Some(ref cookie) = self.session_cookie {
            req = req.header("Cookie", format!("session_id={}", cookie));
        }

        Ok(req.send().await?)
    }

    /// PATCH request with JSON body
    pub async fn patch<T: Serialize>(&self, path: &str, body: &T) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.patch(&url).json(body);

        if let Some(ref cookie) = self.session_cookie {
            req = req.header("Cookie", format!("session_id={}", cookie));
        }

        Ok(req.send().await?)
    }

    /// DELETE request
    pub async fn delete(&self, path: &str) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.delete(&url);

        if let Some(ref cookie) = self.session_cookie {
            req = req.header("Cookie", format!("session_id={}", cookie));
        }

        Ok(req.send().await?)
    }

    /// Check if response status is success
    pub fn is_success(status: StatusCode) -> bool {
        status.is_success()
    }

    /// Get base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Response extension trait for easier assertions
pub trait ResponseExt {
    fn status_code(&self) -> u16;
    fn is_ok(&self) -> bool;
    fn is_created(&self) -> bool;
    fn is_no_content(&self) -> bool;
    fn is_bad_request(&self) -> bool;
    fn is_unauthorized(&self) -> bool;
    fn is_forbidden(&self) -> bool;
    fn is_not_found(&self) -> bool;
}

impl ResponseExt for Response {
    fn status_code(&self) -> u16 {
        self.status().as_u16()
    }

    fn is_ok(&self) -> bool {
        self.status() == StatusCode::OK
    }

    fn is_created(&self) -> bool {
        self.status() == StatusCode::CREATED
    }

    fn is_no_content(&self) -> bool {
        self.status() == StatusCode::NO_CONTENT
    }

    fn is_bad_request(&self) -> bool {
        self.status() == StatusCode::BAD_REQUEST
    }

    fn is_unauthorized(&self) -> bool {
        self.status() == StatusCode::UNAUTHORIZED
    }

    fn is_forbidden(&self) -> bool {
        self.status() == StatusCode::FORBIDDEN
    }

    fn is_not_found(&self) -> bool {
        self.status() == StatusCode::NOT_FOUND
    }
}
