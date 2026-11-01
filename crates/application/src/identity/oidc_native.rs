use super::{
    LoginOutcome,
    entities::oauth_native_flow,
    oidc::{IdTokenAudience, OidcService, verified_subject},
    oidc_provider::Provider,
    repository::oauth,
};
use crate::{ApplicationError, ApplicationResult, database::database_time};
use auth_core::token::{generate_secure_token, hash_token};
use jobs::TraceContext;
use openidconnect::{Nonce, core::CoreIdToken};
use sea_orm::{Set, TransactionTrait};
use uuid::Uuid;

/// Single-use values for one native ID token login.
pub struct NativeAuthorization {
    /// Secret that the native client keeps and returns at completion.
    pub flow: String,
    /// Value that the native client passes unchanged to the provider SDK.
    pub nonce: String,
}

impl OidcService {
    #[tracing::instrument(skip_all, fields(provider = provider_name))]
    pub async fn begin_native_authorization(
        &self,
        provider_name: &str,
    ) -> ApplicationResult<NativeAuthorization> {
        self.native_provider(provider_name)?;
        let flow = generate_secure_token();
        let nonce = generate_secure_token();
        let transaction = self.identity.database.begin().await?;
        oauth::insert_native_flow(
            &transaction,
            oauth_native_flow::ActiveModel {
                id: Set(Uuid::now_v7()),
                flow_hash: Set(hash_token(&flow)),
                nonce_hash: Set(hash_token(&nonce)),
                provider: Set(provider_name.into()),
                expires_at: Set(database_time(&transaction).await? + chrono::Duration::minutes(5)),
            },
        )
        .await?;
        transaction.commit().await?;
        Ok(NativeAuthorization { flow, nonce })
    }

    #[tracing::instrument(skip_all, fields(provider = provider_name, request_id = %trace.request_id))]
    pub async fn complete_native_authorization(
        &self,
        provider_name: &str,
        flow: &str,
        id_token: &str,
        trace: TraceContext,
    ) -> ApplicationResult<LoginOutcome> {
        if flow.len() != 43 || id_token.len() > 8192 {
            return Err(ApplicationError::Unauthenticated);
        }
        let provider = self.native_provider(provider_name)?;
        let transaction = self.identity.database.begin().await?;
        // Consume before verification: a rejected ID token requires a new flow.
        let flow = oauth::consume_native_flow(&transaction, &hash_token(flow), provider_name)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        transaction.commit().await?;
        let id_token: CoreIdToken = id_token
            .parse()
            .map_err(|_| ApplicationError::Unauthenticated)?;
        let nonce_matches = |nonce: Option<&Nonce>| match nonce {
            Some(nonce) if hash_token(nonce.secret()) == flow.nonce_hash => Ok(()),
            _ => Err("nonce mismatch".to_owned()),
        };
        let client = provider.client(&self.http, false).await?;
        let (claims, _) = self
            .verify_id_token(
                provider,
                client,
                &id_token,
                IdTokenAudience::NativeClients,
                nonce_matches,
            )
            .await?;
        let subject = verified_subject(claims)?;
        let transaction = self.identity.database.begin().await?;
        let outcome = self
            .login_with_external_identity(&transaction, provider, subject, claims, trace)
            .await?;
        transaction.commit().await?;
        Ok(outcome)
    }

    fn native_provider(&self, provider_name: &str) -> ApplicationResult<&Provider> {
        self.providers
            .get(provider_name)
            .filter(|provider| provider.supports_native_login())
            .ok_or(ApplicationError::NotFound)
    }
}
