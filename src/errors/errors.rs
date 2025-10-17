use crate::config::db_config::DbConfig;
use crate::errors::protocol::general::{BAD_REQUEST, VALIDATION_ERROR};
use crate::errors::protocol::system::{
    SYS_DATABASE_ERROR, SYS_HASHING_ERROR, SYS_NOT_FOUND, SYS_TOKEN_CREATION_ERROR,
};
use crate::errors::protocol::user::{
    USER_INVALID_PASSWORD, USER_INVALID_TOKEN, USER_NOT_FOUND, USER_TOKEN_EXPIRED,
    USER_UNAUTHORIZED,
};
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    UserUnauthorized,
    UserTokenExpired,
    UserInvalidToken,
    // General
    BadRequestError(String),
    ValidationError(String),
    // System
    SysInternalError(String),
    DatabaseError(String),
    NotFound(String),
    HashingError(String),
    TokenCreationError(String),
}

impl IntoResponse for Errors {
    fn into_response(self) -> Response {
        let (status, code, details) = match self {
            // User
            Errors::UserInvalidPassword => (StatusCode::UNAUTHORIZED, USER_INVALID_PASSWORD, None),
            Errors::UserNotFound => (StatusCode::NOT_FOUND, USER_NOT_FOUND, None),
            Errors::UserUnauthorized => (StatusCode::UNAUTHORIZED, USER_UNAUTHORIZED, None),
            Errors::UserTokenExpired => (StatusCode::UNAUTHORIZED, USER_TOKEN_EXPIRED, None),
            Errors::UserInvalidToken => (StatusCode::UNAUTHORIZED, USER_INVALID_TOKEN, None),
            // General
            Errors::BadRequestError(msg) => (StatusCode::BAD_REQUEST, BAD_REQUEST, Some(msg)),
            Errors::ValidationError(msg) => (StatusCode::BAD_REQUEST, VALIDATION_ERROR, Some(msg)),
            // System
            Errors::SysInternalError(_)
             | Errors::DatabaseError(msg) => (
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
            Errors::TokenCreationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                SYS_TOKEN_CREATION_ERROR,
                Some(msg),
            ),
        };

        let is_dev = DbConfig::get().is_dev;

        let body = ErrorResponse {
            status: status.as_u16(),
            code: code.to_string(),
            details: if is_dev { details } else { None },
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
