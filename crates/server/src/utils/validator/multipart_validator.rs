use crate::errors::errors::Errors;
use axum::extract::{FromRequest, Multipart, Request};
use validator::Validate;

#[derive(Debug)]
pub struct ValidatedMultipart<T>(pub T);

/// Trait for types that can be parsed from multipart form data
pub trait FromMultipart: Sized + Send {
    fn from_multipart(multipart: Multipart) -> impl Future<Output = Result<Self, Errors>> + Send;
}

impl<T, S> FromRequest<S> for ValidatedMultipart<T>
where
    T: FromMultipart + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Errors;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| Errors::BadRequestError(format!("Invalid multipart data: {}", e)))?;

        let value = T::from_multipart(multipart).await?;

        value
            .validate()
            .map_err(|e| Errors::ValidationError(e.to_string()))?;

        Ok(ValidatedMultipart(value))
    }
}
