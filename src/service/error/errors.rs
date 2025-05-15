use crate::service::error::protocol::general::{BAD_REQUEST, VALIDATION_ERROR};
use crate::service::error::protocol::system::{
    SYS_DATABASE_ERROR, SYS_HASHING_ERROR, SYS_NOT_FOUND,
};
use crate::service::error::protocol::user::{USER_INVALID_PASSWORD, USER_NOT_FOUND};
use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tracing::error;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub status: u16,
    pub code: String,
    pub details: Option<String>,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

pub enum Errors {
    // User
    UserInvalidPassword,
    UserNotFound,
    // General
    BadRequestError(String),
    ValidationError(String),
    // System
    DatabaseError(String),
    NotFound(String),
    HashingError(String),
}

impl IntoResponse for Errors {
    fn into_response(self) -> Response {
        let (status, code, details) = match self {
            // User
            Errors::UserInvalidPassword => (StatusCode::UNAUTHORIZED, USER_INVALID_PASSWORD, None),
            Errors::UserNotFound => (StatusCode::NOT_FOUND, USER_NOT_FOUND, None),
            // General
            Errors::BadRequestError(msg) => (StatusCode::BAD_REQUEST, BAD_REQUEST, Some(msg)),
            Errors::ValidationError(msg) => (StatusCode::BAD_REQUEST, VALIDATION_ERROR, Some(msg)),
            // System
            Errors::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                SYS_DATABASE_ERROR,
                Some(msg),
            ),
            Errors::NotFound(msg) => (StatusCode::NOT_FOUND, SYS_NOT_FOUND, Some(msg)),
            Errors::HashingError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                SYS_HASHING_ERROR,
                Some(msg),
            ),
        };

        let body = ErrorResponse {
            status: status.as_u16(),
            code: code.to_string(),
            details,
        };

        (status, Json(body)).into_response()
    }
}

pub async fn handler_404<B>(req: Request<B>) -> impl IntoResponse {
    let path = req.uri().path();
    let method = req.method().to_string();

    error!(
        "404 Error: Requested path {} with method {} not found.",
        path, method
    );

    Errors::NotFound("The requested resource was not found.".to_string())
}
