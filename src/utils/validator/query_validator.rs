use crate::errors::errors::Errors;
use axum::extract::rejection::QueryRejection;
use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;
use validator::Validate;

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Query<T>: FromRequestParts<S, Rejection = QueryRejection>,
{
    type Rejection = Errors;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| Errors::BadRequestError(e.to_string()))?;
        value
            .validate()
            .map_err(|e| Errors::ValidationError(e.to_string()))?;
        Ok(ValidatedQuery(value))
    }
}