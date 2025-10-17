use crate::dto::auth::internal::access_token::AccessTokenClaims;
use crate::dto::user::request::create::CreateUserRequest;
use crate::dto::user::response::info::UserInfoResponse;
use crate::middleware::auth::access_jwt_auth;
use crate::errors::errors::Errors;
use crate::service::user::user::{
    service_create_user, service_get_user_by_handle, service_get_user_by_uuid,
};
use crate::service::validator::json_validator::ValidatedJson;
use crate::state::AppState;
use axum::Router;
use axum::extract::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use tracing::info;

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/user/{handle}", get(get_user))
        .route("/user", post(create_user))
        // 보호된 사용자 프로필 API
        .route(
            "/user/profile",
            get(get_profile).route_layer(axum::middleware::from_fn(access_jwt_auth)),
        )
}

#[utoipa::path(
    get,
    path = "/v0/user/{handle}",
    params(
        ("handle" = String, Path, description = "User handle")
    ),
    responses(
        (status = StatusCode::OK, description = "Successfully retrieved user information", body = UserInfoResponse),
        (status = StatusCode::NOT_FOUND, description = "User not found"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error")
    ),
    tag = "User"
)]
pub async fn get_user(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> Result<UserInfoResponse, Errors> {
    info!("Received GET request for user with ID: {}", handle);

    let user = service_get_user_by_handle(&state.conn, &handle).await?;
    Ok(user)
}

// POST /user
#[utoipa::path(
    post,
    path = "/v0/user",
    request_body = CreateUserRequest,
    responses(
        (status = StatusCode::CREATED, description = "User created successfully"),
        (status = StatusCode::BAD_REQUEST, description = "Invalid input"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error")
    ),
    tag = "User"
)]
pub async fn create_user(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserRequest>,
) -> Result<impl IntoResponse, Errors> {
    info!("Received POST request to create user: {:?}", payload);

    service_create_user(&state.conn, payload).await?;

    Ok(StatusCode::NO_CONTENT)
}

// 보호된 API - 사용자 프로필 조회
#[utoipa::path(
    get,
    path = "/v0/user/profile",
    responses(
        (status = StatusCode::OK, description = "Successfully retrieved user profile", body = UserInfoResponse),
        (status = StatusCode::UNAUTHORIZED, description = "Unauthorized"),
        (status = StatusCode::NOT_FOUND, description = "User not found"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "User"
)]
pub async fn get_profile(
    state: State<AppState>,
    Extension(claims): Extension<AccessTokenClaims>,
) -> Result<UserInfoResponse, Errors> {
    let user_uuid = claims.sub.clone();

    // 예시 응답 (실제로는 DB에서 조회)
    let user = service_get_user_by_uuid(&state.conn, &user_uuid).await?;

    Ok(user)
}
