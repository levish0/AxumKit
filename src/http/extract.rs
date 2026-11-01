use super::{HttpState, client_address::client_address, error::ApiError};
use application::config::SecretString;
use application::{ApplicationError, identity::AuthenticatedSession};
use axum::{
    Json,
    extract::{ConnectInfo, FromRequest, FromRequestParts, Path, Query, Request},
    http::{HeaderMap, StatusCode, request::Parts},
};
use cookie::Cookie;
use serde::de::DeserializeOwned;
use validator::Validate;

pub struct ApiKeyCredential(pub(super) SecretString);
impl FromRequestParts<HttpState> for ApiKeyCredential {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &HttpState,
    ) -> Result<Self, Self::Rejection> {
        if read_cookie(&parts.headers, state.settings.session_cookie_name())?.is_some() {
            return Err(ApiError::invalid_input());
        }
        let value = read_bearer_token(&parts.headers)?
            .filter(|value| value.len() <= 128)
            .ok_or(ApplicationError::Unauthenticated)?;
        Ok(Self(SecretString::new(value)))
    }
}

pub struct ApiJson<T>(pub T);
impl<T: DeserializeOwned + Validate + Send, S: Send + Sync> FromRequest<S> for ApiJson<T> {
    type Rejection = ApiError;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(request, state)
            .await
            .map_err(|error| {
                if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
                    ApiError::new(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "body_too_large",
                        "The request body is too large.",
                    )
                } else {
                    ApiError::invalid_input()
                }
            })?;
        // Validation errors may contain submitted secrets; never log or serialize them.
        value.validate().map_err(|_| ApiError::invalid_input())?;
        Ok(Self(value))
    }
}
pub struct ApiQuery<T>(pub T);
impl<T: DeserializeOwned + Validate + Send, S: Send + Sync> FromRequestParts<S> for ApiQuery<T> {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::invalid_input())?;
        value.validate().map_err(|_| ApiError::invalid_input())?;
        Ok(Self(value))
    }
}
pub struct ApiPath<T>(pub T);
impl<T: DeserializeOwned + Send, S: Send + Sync> FromRequestParts<S> for ApiPath<T> {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Path::<T>::from_request_parts(parts, state)
            .await
            .map(|Path(value)| Self(value))
            .map_err(|_| ApiError::invalid_input())
    }
}

pub struct Session(pub AuthenticatedSession);
impl FromRequestParts<HttpState> for Session {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &HttpState,
    ) -> Result<Self, Self::Rejection> {
        let token = session_token(&parts.headers, state.settings.session_cookie_name())?
            .ok_or(ApplicationError::Unauthenticated)?;
        Ok(Self(state.app.identity.authenticate_session(&token).await?))
    }
}

/// Client IP address under the Cloudflare Tunnel and SSR proxy contract.
pub struct ClientAddress(pub Option<std::net::IpAddr>);
impl FromRequestParts<HttpState> for ClientAddress {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &HttpState,
    ) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<std::net::SocketAddr>>()
            .map(|peer| peer.0.ip());
        client_address(
            peer,
            &parts.headers,
            state.settings.internal_proxy_secret.as_ref(),
        )
        .map(Self)
    }
}

pub fn read_cookie(headers: &HeaderMap, name: &str) -> Result<Option<String>, ApiError> {
    let mut result = None;
    for value in headers.get_all(axum::http::header::COOKIE) {
        let value = value.to_str().map_err(|_| ApiError::invalid_input())?;
        for cookie in Cookie::split_parse(value) {
            let cookie = cookie.map_err(|_| ApiError::invalid_input())?;
            if cookie.name() == name {
                if result.is_some() {
                    return Err(ApiError::invalid_input());
                }
                result = Some(cookie.value().to_owned());
            }
        }
    }
    Ok(result)
}

fn session_token(headers: &HeaderMap, cookie_name: &str) -> Result<Option<String>, ApiError> {
    let cookie = read_cookie(headers, cookie_name)?;
    let bearer = read_bearer_token(headers)?;
    if cookie.is_some() && bearer.is_some() {
        return Err(ApiError::invalid_input());
    }
    Ok(bearer.map(str::to_owned).or(cookie))
}

pub fn read_bearer_token(headers: &HeaderMap) -> Result<Option<&str>, ApiError> {
    let mut authorization = headers.get_all(axum::http::header::AUTHORIZATION).iter();
    let first = authorization.next();
    if authorization.next().is_some() {
        return Err(ApiError::invalid_input());
    }
    first
        .map(|value| {
            let (scheme, token) = value
                .to_str()
                .ok()
                .and_then(|value| value.split_once(' '))
                .ok_or(ApplicationError::Unauthenticated)?;
            let token = token.trim_start_matches(' ');
            if !scheme.eq_ignore_ascii_case("Bearer")
                || token.is_empty()
                || token.bytes().any(|byte| byte.is_ascii_whitespace())
            {
                return Err(ApiError::from(ApplicationError::Unauthenticated));
            }
            Ok(token)
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, header};

    #[test]
    fn bearer_scheme_is_case_insensitive_and_ambiguous_credentials_are_rejected() {
        let mut headers = HeaderMap::new();
        assert!(read_bearer_token(&headers).unwrap().is_none());
        for value in ["Bearer token", "bearer token", "bEaReR   token"] {
            headers.insert(header::AUTHORIZATION, HeaderValue::from_static(value));
            assert_eq!(read_bearer_token(&headers).unwrap(), Some("token"));
        }
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("session=cookie-token"),
        );
        assert_eq!(
            session_token(&headers, "session").unwrap_err().status,
            StatusCode::BAD_REQUEST
        );
        headers.remove(header::COOKIE);
        for value in ["Basic token", "Bearer", "Bearer ", "Bearer two tokens"] {
            headers.insert(header::AUTHORIZATION, HeaderValue::from_static(value));
            assert_eq!(
                read_bearer_token(&headers).unwrap_err().status,
                StatusCode::UNAUTHORIZED
            );
        }
        headers.append(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer another"),
        );
        assert_eq!(
            read_bearer_token(&headers).unwrap_err().status,
            StatusCode::BAD_REQUEST
        );
    }
}
