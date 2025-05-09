use crate::service::error::protocol::general::{BAD_REQUEST, VALIDATION_ERROR};
use crate::service::error::protocol::system::{DATABASE_ERROR, INTERNAL_ERROR, NOT_FOUND};
use crate::service::error::protocol::user::{EMAIL_EXISTS, USER_NOT_FOUND, USERNAME_EXISTS};
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
    EmailExists,
    UsernameTaken,
    UserNotFound,
    DatabaseError(String),
    BadRequestError(String),
    ValidationError(String),
    InternalError(String),
    NotFound(String),
}

impl IntoResponse for Errors {
    fn into_response(self) -> Response {
        let (status, code, details) = match self {
            Errors::EmailExists => (StatusCode::BAD_REQUEST, EMAIL_EXISTS, None),
            Errors::UsernameTaken => (StatusCode::BAD_REQUEST, USERNAME_EXISTS, None),
            Errors::UserNotFound => (StatusCode::NOT_FOUND, USER_NOT_FOUND, None),
            Errors::DatabaseError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, DATABASE_ERROR, Some(msg))
            }
            Errors::BadRequestError(msg) => (StatusCode::BAD_REQUEST, BAD_REQUEST, Some(msg)),
            Errors::ValidationError(msg) => (StatusCode::BAD_REQUEST, VALIDATION_ERROR, Some(msg)),
            Errors::InternalError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR, Some(msg))
            }
            Errors::NotFound(msg) => (StatusCode::NOT_FOUND, NOT_FOUND, Some(msg)),
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