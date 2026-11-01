use super::*;
use application::identity::{
    AuthenticatedSession, IdentityService,
    entities::{audit_event, rate_limit, recovery_code},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, QueryFilter, Set, sea_query::Expr};

async fn enrolled_account(
    database: &TestDatabase,
    identity: &IdentityService,
) -> anyhow::Result<(String, Uuid, Vec<String>)> {
    let (actor, _) = database.verified_account(identity).await?;
    let email = identity.get_account(&actor).await?.email;
    let enrollment = identity.begin_totp_enrollment(&actor).await?;
    let authenticator = totp_rs::Totp::from_url(&enrollment.authenticator_url)?;
    let recovery_codes = identity
        .confirm_totp_enrollment(
            &actor,
            &enrollment.challenge,
            &authenticator.generate_current().to_string(),
            TraceContext::default(),
        )
        .await?;
    Ok((email, actor.account_id(), recovery_codes))
}

async fn login_with_recovery(
    identity: &IdentityService,
    email: &str,
    code: &str,
) -> anyhow::Result<AuthenticatedSession> {
    let challenge = super::mfa_challenge(identity, email).await?;
    let session = identity
        .complete_mfa_login(
            &challenge,
            MfaProof::RecoveryCode(code.into()),
            TraceContext::default(),
        )
        .await?;
    Ok(identity.authenticate_session(&session.token).await?)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn privileged_accounts_require_mfa_at_login_session_and_final_authority() -> anyhow::Result<()>
{
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (actor, credentials) = database.verified_account(&identity).await?;
    let email = identity.get_account(&actor).await?.email;
    assert!(matches!(
        identity
            .set_administrator_by_operator(actor.account_id(), true, TraceContext::default())
            .await,
        Err(ApplicationError::InvalidInput(_))
    ));
    account::Entity::update_many()
        .col_expr(account::Column::Administrator, Expr::value(true))
        .filter(account::Column::Id.eq(actor.account_id()))
        .exec(&database.connection)
        .await?;
    assert!(matches!(
        identity.authenticate_session(&credentials.token).await,
        Err(ApplicationError::Unauthenticated)
    ));
    assert!(matches!(
        identity.get_account(&actor).await,
        Err(ApplicationError::Unauthenticated)
    ));
    assert!(matches!(
        identity
            .login_with_password(&email, TEST_PASSWORD.into(), TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    identity
        .set_administrator_by_operator(actor.account_id(), false, TraceContext::default())
        .await?;
    assert!(
        identity
            .authenticate_session(&credentials.token)
            .await
            .is_err()
    );

    let (email, account_id, codes) = enrolled_account(&database, &identity).await?;
    let before_promotion = login_with_recovery(&identity, &email, &codes[0]).await?;
    identity
        .set_administrator_by_operator(account_id, true, TraceContext::default())
        .await?;
    assert!(identity.get_account(&before_promotion).await.is_err());
    let administrator = login_with_recovery(&identity, &email, &codes[1]).await?;
    assert!(matches!(
        identity
            .disable_totp(
                &administrator,
                MfaProof::RecoveryCode(codes[2].clone()),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Forbidden)
    ));
    identity
        .set_administrator_by_operator(account_id, false, TraceContext::default())
        .await?;
    assert!(identity.get_account(&administrator).await.is_err());
    let actor = login_with_recovery(&identity, &email, &codes[2]).await?;
    let (promotion, removal) = tokio::join!(
        identity.set_administrator_by_operator(account_id, true, TraceContext::default()),
        identity.disable_totp(
            &actor,
            MfaProof::RecoveryCode(codes[3].clone()),
            TraceContext::default()
        )
    );
    assert_eq!(
        usize::from(promotion.is_ok()) + usize::from(removal.is_ok()),
        1
    );
    let account = account::Entity::find_by_id(account_id)
        .one(&database.connection)
        .await?
        .unwrap();
    assert!(!account.administrator || account.totp_secret.is_some());
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn concurrent_recovery_regeneration_returns_one_generation_and_revokes_credentials()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (email, account_id, codes) = enrolled_account(&database, &identity).await?;
    let actor = login_with_recovery(&identity, &email, &codes[0]).await?;
    session::Entity::update_many()
        .col_expr(
            session::Column::AuthenticatedAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::minutes(6)),
        )
        .filter(session::Column::Id.eq(actor.session_id()))
        .exec(&database.connection)
        .await?;
    assert!(matches!(
        identity
            .regenerate_recovery_codes(
                &actor,
                MfaProof::RecoveryCode(codes[1].clone()),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Forbidden)
    ));
    session::Entity::update_many()
        .col_expr(
            session::Column::AuthenticatedAt,
            Expr::value(chrono::Utc::now()),
        )
        .filter(session::Column::Id.eq(actor.session_id()))
        .exec(&database.connection)
        .await?;
    let pending_challenge = super::mfa_challenge(&identity, &email).await?;
    let (first, second) = tokio::join!(
        identity.regenerate_recovery_codes(
            &actor,
            MfaProof::RecoveryCode(codes[1].clone()),
            TraceContext::default()
        ),
        identity.regenerate_recovery_codes(
            &actor,
            MfaProof::RecoveryCode(codes[2].clone()),
            TraceContext::default()
        )
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let replacements = first.or(second)?;
    assert_eq!(replacements.len(), 10);
    assert!(identity.get_account(&actor).await.is_err());
    assert!(
        identity
            .complete_mfa_login(
                &pending_challenge,
                MfaProof::RecoveryCode(codes[3].clone()),
                TraceContext::default()
            )
            .await
            .is_err()
    );
    let stored = recovery_code::Entity::find()
        .filter(recovery_code::Column::AccountId.eq(account_id))
        .all(&database.connection)
        .await?;
    assert_eq!(stored.len(), 10);
    assert!(replacements.iter().all(|code| {
        stored
            .iter()
            .any(|row| row.code_hash == auth_core::token::hash_token(code))
    }));
    assert!(
        login_with_recovery(&identity, &email, &codes[3])
            .await
            .is_err()
    );
    login_with_recovery(&identity, &email, &replacements[0]).await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn configured_rate_limits_above_601_remain_shared_and_do_not_reset_counters()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut settings = test_support::identity_settings()?;
    settings.authentication_client_limit_per_minute = 602;
    let high = IdentityService::new(
        database.connection.clone(),
        settings.clone(),
        std::sync::Arc::new(application::authorization::Authorization::new()?),
    )
    .await?;
    settings.authentication_client_limit_per_minute = 2;
    let low = IdentityService::new(
        database.connection.clone(),
        settings,
        std::sync::Arc::new(application::authorization::Authorization::new()?),
    )
    .await?;
    high.limit_client_authentication("192.0.2.20").await?;
    rate_limit::Entity::update_many()
        .col_expr(rate_limit::Column::Attempts, Expr::value(600))
        .exec(&database.connection)
        .await?;
    assert!(matches!(
        low.limit_client_authentication("192.0.2.20").await,
        Err(ApplicationError::RateLimited)
    ));
    high.limit_client_authentication("192.0.2.20").await?;
    assert!(matches!(
        high.limit_client_authentication("192.0.2.20").await,
        Err(ApplicationError::RateLimited)
    ));
    assert_eq!(
        rate_limit::Entity::find()
            .one(&database.connection)
            .await?
            .unwrap()
            .attempts,
        603
    );
    high.limit_client_authentication("192.0.2.21").await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn audit_retention_is_bounded_and_preserves_recent_events() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let old = chrono::Utc::now() - chrono::Duration::days(181);
    audit_event::Entity::insert_many((0..1002).map(|_| audit_event::ActiveModel {
        id: Set(Uuid::now_v7()),
        account_id: Set(None),
        event: Set("fixture".into()),
        request_id: Set(Uuid::now_v7()),
        created_at: Set(old),
        ..Default::default()
    }))
    .exec(&database.connection)
    .await?;
    let recent_id = Uuid::now_v7();
    audit_event::ActiveModel {
        id: Set(recent_id),
        account_id: Set(None),
        event: Set("fixture".into()),
        request_id: Set(Uuid::now_v7()),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(&database.connection)
    .await?;
    assert!(identity.cleanup_audit_events(0).await.is_err());
    assert_eq!(identity.cleanup_audit_events(180).await?, 1000);
    assert_eq!(identity.cleanup_audit_events(180).await?, 2);
    assert!(
        audit_event::Entity::find_by_id(recent_id)
            .one(&database.connection)
            .await?
            .is_some()
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn recovery_regeneration_keeps_the_consumed_totp_step_after_version_change()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (actor, _) = database.verified_account(&identity).await?;
    let email = identity.get_account(&actor).await?.email;
    let enrollment = identity.begin_totp_enrollment(&actor).await?;
    let authenticator = totp_rs::Totp::from_url(&enrollment.authenticator_url)?;
    let codes = identity
        .confirm_totp_enrollment(
            &actor,
            &enrollment.challenge,
            &authenticator.generate_current().to_string(),
            TraceContext::default(),
        )
        .await?;
    let actor = login_with_recovery(&identity, &email, &codes[0]).await?;
    let next_step = chrono::Utc::now().timestamp() / 30 + 1;
    let proof = authenticator.generate(next_step as u64 * 30).to_string();
    identity
        .regenerate_recovery_codes(
            &actor,
            MfaProof::Totp(proof.clone()),
            TraceContext::default(),
        )
        .await?;
    let account = account::Entity::find_by_id(actor.account_id())
        .one(&database.connection)
        .await?
        .unwrap();
    assert_eq!(account.totp_last_step, next_step);
    let challenge = super::mfa_challenge(&identity, &email).await?;
    assert!(matches!(
        identity
            .complete_mfa_login(&challenge, MfaProof::Totp(proof), TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    database.finish().await
}
