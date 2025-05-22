use crate::service::auth::jwt::{decode_access_token, decode_refresh_token};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::TokenData;

pub async fn jwt_auth<T, F>(
    mut req: Request<Body>,
    next: Next,
    decode_fn: F,
) -> Result<Response, StatusCode>
where
    T: Send + Sync + Clone + 'static,
    F: Fn(&str) -> Result<TokenData<T>, jsonwebtoken::errors::Error>,
{
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let token = if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            header.trim_start_matches("Bearer ")
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let token_data = match decode_fn(token) {
        Ok(data) => data,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}

pub async fn access_jwt_auth(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    jwt_auth(req, next, decode_access_token).await
}

pub async fn refresh_jwt_auth(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    jwt_auth(req, next, decode_refresh_token).await
}
