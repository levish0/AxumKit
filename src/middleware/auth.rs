use crate::service::auth::jwt::{decode_access_token, decode_refresh_token};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

// Access Token 검증 미들웨어
pub async fn jwt_auth(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let token = if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            header.trim_start_matches("Bearer ").to_string()
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let token_data = match decode_access_token(&token) {
        Ok(data) => data,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };
    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}

// Refresh Token 검증 미들웨어
pub async fn refresh_jwt_auth(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let token = if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            header.trim_start_matches("Bearer ").to_string()
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let token_data = match decode_refresh_token(&token) {
        Ok(data) => data,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };
    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}
