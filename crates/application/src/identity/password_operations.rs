use crate::{ApplicationError, ApplicationResult};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub(super) struct PasswordOperations {
    permits: Arc<Semaphore>,
    dummy_hash: String,
}
impl PasswordOperations {
    pub async fn new(concurrency: usize) -> ApplicationResult<Self> {
        let dummy_hash =
            tokio::task::spawn_blocking(|| auth_core::password::hash("timing equalization value"))
                .await
                .map_err(ApplicationError::internal)?
                .map_err(ApplicationError::internal)?;
        Ok(Self {
            permits: Arc::new(Semaphore::new(concurrency)),
            dummy_hash,
        })
    }

    pub async fn hash(&self, password: String) -> ApplicationResult<String> {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(ApplicationError::internal)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            auth_core::password::hash(&password)
        })
        .await
        .map_err(ApplicationError::internal)?
        .map_err(ApplicationError::internal)
    }

    pub async fn verify(
        &self,
        password: String,
        expected_hash: Option<&str>,
    ) -> ApplicationResult<bool> {
        let account_has_password = expected_hash.is_some();
        let expected_hash = expected_hash.unwrap_or(&self.dummy_hash).to_owned();
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(ApplicationError::internal)?;
        let verified = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            auth_core::password::verify(&password, &expected_hash)
        })
        .await
        .map_err(ApplicationError::internal)?
        .map_err(ApplicationError::internal)?;
        Ok(account_has_password && verified)
    }
}
