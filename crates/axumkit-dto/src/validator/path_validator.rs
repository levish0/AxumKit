use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use axumkit_errors::errors::Errors;
use serde::de::DeserializeOwned;
use validator::Validate;

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedPath<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Errors;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| Errors::BadRequestError(e.to_string()))?;
        value
            .validate()
            .map_err(|e| Errors::ValidationError(e.to_string()))?;
        Ok(ValidatedPath(value))
    }
}
