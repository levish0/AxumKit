use application::{
    ApplicationError,
    identity::{
        LoginOutcome, MfaProof,
        entities::{account, challenge, mail_delivery, session},
    },
    organizations::{OrganizationService, entities::organization},
};
use auth_core::token::generate_secure_token;
use jobs::{TraceContext, entities::job};
use sea_orm::{EntityTrait, PaginatorTrait, TransactionTrait};
use test_support::{TEST_PASSWORD, TestDatabase};
use uuid::Uuid;

#[path = "identity/hardening.rs"]
mod hardening;

#[path = "identity/notifications.rs"]
mod notifications;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn registration_persists_encrypted_mail_and_durable_job() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    identity
        .register_account(
            "Owner@Example.test",
            TEST_PASSWORD.into(),
            TraceContext::default(),
        )
        .await?;
    assert_eq!(
        account::Entity::find().count(&database.connection).await?,
        1
    );
    assert_eq!(
        challenge::Entity::find()
            .count(&database.connection)
            .await?,
        1
    );
    let delivery = mail_delivery::Entity::find()
        .one(&database.connection)
        .await?
        .unwrap();
    assert!(!delivery.ciphertext.as_ref().unwrap().contains("token="));
    let queued = job::Entity::find()
        .one(&database.connection)
        .await?
        .unwrap();
    assert_eq!(queued.payload.as_object().unwrap().len(), 1);
    assert_eq!(queued.id.get_version_num(), 7);
    assert_eq!(delivery.id.get_version_num(), 7);
    assert!(matches!(
        identity
            .login_with_password(
                "owner@example.test",
                TEST_PASSWORD.into(),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    identity
        .register_account(
            "owner@example.test",
            TEST_PASSWORD.into(),
            TraceContext::default(),
        )
        .await?;
    assert_eq!(
        account::Entity::find().count(&database.connection).await?,
        1
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn caller_transaction_rolls_back_domain_and_job_together() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (actor, _) = database.verified_account(&identity).await?;
    let organizations = OrganizationService::new(identity);
    let transaction = database.connection.begin().await?;
    organizations
        .create_organization_in_transaction(&transaction, &actor, "Rolled back".into())
        .await?;
    jobs::enqueue(
        &transaction,
        Uuid::now_v7(),
        "test.rollback",
        &serde_json::json!({}),
        TraceContext::default(),
    )
    .await?;
    transaction.rollback().await?;
    assert_eq!(
        organization::Entity::find()
            .count(&database.connection)
            .await?,
        0
    );
    assert_eq!(job::Entity::find().count(&database.connection).await?, 0);
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn authentication_cleanup_preserves_a_concurrent_idle_extension() -> anyhow::Result<()> {
    use sea_orm::{
        ColumnTrait, ConnectionTrait, ExprTrait, QueryFilter, QuerySelect,
        sea_query::{Alias, Expr, Func, Query},
    };

    async fn database_time(
        connection: &impl ConnectionTrait,
    ) -> Result<chrono::DateTime<chrono::Utc>, sea_orm::DbErr> {
        connection
            .query_one(
                &Query::select()
                    .expr_as(Func::cust("clock_timestamp"), "database_time")
                    .to_owned(),
            )
            .await?
            .expect("database clock query must return one row")
            .try_get("", "database_time")
    }

    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (actor, credentials) = database.verified_account(&identity).await?;
    let (expired_actor, _) = database.verified_account(&identity).await?;
    let now = database_time(&database.connection).await?;
    let previous_idle_deadline = now - chrono::Duration::seconds(1);
    session::Entity::update_many()
        .col_expr(
            session::Column::IdleExpiresAt,
            Expr::value(previous_idle_deadline),
        )
        .filter(session::Column::Id.eq(actor.session_id()))
        .exec(&database.connection)
        .await?;
    session::Entity::update_many()
        .col_expr(
            session::Column::ExpiresAt,
            Expr::value(now - chrono::Duration::seconds(1)),
        )
        .filter(session::Column::Id.eq(expired_actor.session_id()))
        .exec(&database.connection)
        .await?;

    // Model the cleaner's expired snapshot while an idle extension remains uncommitted.
    let extension = database.connection.begin().await?;
    account::Entity::find_by_id(actor.account_id())
        .lock_exclusive()
        .one(&extension)
        .await?;
    session::Entity::update_many()
        .col_expr(
            session::Column::IdleExpiresAt,
            Expr::value(now + chrono::Duration::hours(1)),
        )
        .filter(session::Column::Id.eq(actor.session_id()))
        .exec(&extension)
        .await?;
    let cleanup_identity = identity.clone();
    let cleanup =
        tokio::spawn(async move { cleanup_identity.cleanup_expired_authentication().await });
    // This isolated database has one blocked writer: cleanup waiting on the idle extension.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let waiting = database
                .connection
                .query_one(
                    &Query::select()
                        .expr_as(Func::count(Expr::col(Alias::new("pid"))), "waiting")
                        .from(Alias::new("pg_stat_activity"))
                        .and_where(
                            Expr::col(Alias::new("datname")).eq(Func::cust("current_database")),
                        )
                        .and_where(Expr::col(Alias::new("wait_event_type")).eq("Lock"))
                        .to_owned(),
                )
                .await?
                .unwrap()
                .try_get::<i64>("", "waiting")?;
            if waiting > 0 {
                return anyhow::Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await??;
    extension.commit().await?;
    assert_eq!(cleanup.await??, 1);
    identity.authenticate_session(&credentials.token).await?;
    assert!(
        session::Entity::find_by_id(expired_actor.session_id())
            .one(&database.connection)
            .await?
            .is_none()
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn session_rotation_is_single_use_and_revocation_is_shared() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let first = database.identity_service().await?;
    let second = database.identity_service().await?;
    let (actor, credentials) = database.verified_account(&first).await?;
    let (left, right) = tokio::join!(
        first.rotate_session(&actor, TraceContext::default()),
        second.rotate_session(&actor, TraceContext::default())
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert!(matches!(
        second.authenticate_session(&credentials.token).await,
        Err(ApplicationError::Unauthenticated)
    ));
    let rotated = left.or(right)?;
    let refreshed = second.authenticate_session(&rotated.token).await?;
    second
        .revoke_session(&refreshed, rotated.session_id, TraceContext::default())
        .await?;
    assert!(matches!(
        first.authenticate_session(&rotated.token).await,
        Err(ApplicationError::Unauthenticated)
    ));
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn password_reset_consumes_once_and_revokes_every_session() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (actor, old_session) = database.verified_account(&identity).await?;
    let token = generate_secure_token();
    database
        .insert_challenge(
            actor.account_id(),
            application::identity::ChallengePurpose::ResetPassword,
            &token,
        )
        .await?;
    let (left, right) = tokio::join!(
        identity.reset_password(
            &token,
            "replacement password one".into(),
            TraceContext::default()
        ),
        identity.reset_password(
            &token,
            "replacement password two".into(),
            TraceContext::default()
        )
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert!(
        identity
            .authenticate_session(&old_session.token)
            .await
            .is_err()
    );
    assert_eq!(
        session::Entity::find().count(&database.connection).await?,
        0
    );
    let email = account::Entity::find_by_id(actor.account_id())
        .one(&database.connection)
        .await?
        .unwrap()
        .email;
    assert!(matches!(
        identity
            .login_with_password(&email, TEST_PASSWORD.into(), TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn failed_authentication_attempts_survive_rollback_across_instances() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let first = database.identity_service().await?;
    let second = database.identity_service().await?;
    for attempt in 0..10 {
        let identity = if attempt % 2 == 0 { &first } else { &second };
        assert!(matches!(
            identity
                .login_with_password(
                    "missing@example.test",
                    "incorrect password".into(),
                    TraceContext::default()
                )
                .await,
            Err(ApplicationError::Unauthenticated)
        ));
    }
    assert!(matches!(
        second
            .login_with_password(
                "missing@example.test",
                TEST_PASSWORD.into(),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::RateLimited)
    ));
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn totp_and_recovery_codes_are_single_use_across_challenges() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (actor, original_session) = database.verified_account(&identity).await?;
    let enrollment = identity.begin_totp_enrollment(&actor).await?;
    let authenticator = totp_rs::Totp::from_url(&enrollment.authenticator_url)?;
    let code = authenticator.generate_current().to_string();
    let recovery_codes = identity
        .confirm_totp_enrollment(
            &actor,
            &enrollment.challenge,
            &code,
            TraceContext::default(),
        )
        .await?;
    assert!(
        identity
            .authenticate_session(&original_session.token)
            .await
            .is_err()
    );
    let email = account::Entity::find_by_id(actor.account_id())
        .one(&database.connection)
        .await?
        .unwrap()
        .email;
    let left = mfa_challenge(&identity, &email).await?;
    let right = mfa_challenge(&identity, &email).await?;
    assert!(
        identity
            .complete_mfa_login(&left, MfaProof::Totp(code), TraceContext::default())
            .await
            .is_err()
    );
    let (first, second) = tokio::join!(
        identity.complete_mfa_login(
            &left,
            MfaProof::RecoveryCode(recovery_codes[0].clone()),
            TraceContext::default()
        ),
        identity.complete_mfa_login(
            &right,
            MfaProof::RecoveryCode(recovery_codes[0].clone()),
            TraceContext::default()
        )
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let session = first.or(second)?;
    let authenticated = identity.authenticate_session(&session.token).await?;
    assert!(identity.get_account(&authenticated).await?.mfa_enabled);
    identity
        .disable_totp(
            &authenticated,
            MfaProof::RecoveryCode(recovery_codes[1].clone()),
            TraceContext::default(),
        )
        .await?;
    assert!(identity.authenticate_session(&session.token).await.is_err());
    assert!(matches!(
        identity
            .login_with_password(&email, TEST_PASSWORD.into(), TraceContext::default())
            .await?,
        LoginOutcome::Session(_)
    ));
    database.finish().await
}

async fn mfa_challenge(
    identity: &application::identity::IdentityService,
    email: &str,
) -> anyhow::Result<String> {
    match identity
        .login_with_password(email, TEST_PASSWORD.into(), TraceContext::default())
        .await?
    {
        LoginOutcome::MfaRequired { challenge } => Ok(challenge),
        _ => anyhow::bail!("MFA was bypassed"),
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn session_ownership_password_change_and_deactivation_recheck_authority() -> anyhow::Result<()>
{
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (owner, credentials) = database.verified_account(&identity).await?;
    let (_, other_credentials) = database.verified_account(&identity).await?;
    let account = identity.get_account(&owner).await?;
    let sessions = identity.list_sessions(&owner, Some(1), None).await?;
    assert_eq!(sessions.items.len(), 1);
    assert_eq!(sessions.items[0].id, credentials.session_id);
    assert!(matches!(
        identity
            .revoke_session(
                &owner,
                other_credentials.session_id,
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::NotFound)
    ));
    let new_password = "changed strong test passphrase";
    assert!(matches!(
        identity
            .change_password(
                &owner,
                "wrong password".into(),
                new_password.into(),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    identity.authenticate_session(&credentials.token).await?;
    identity
        .change_password(
            &owner,
            TEST_PASSWORD.into(),
            new_password.into(),
            TraceContext::default(),
        )
        .await?;
    assert!(
        identity
            .authenticate_session(&credentials.token)
            .await
            .is_err()
    );
    assert!(
        identity
            .login_with_password(
                &account.email,
                TEST_PASSWORD.into(),
                TraceContext::default()
            )
            .await
            .is_err()
    );
    let LoginOutcome::Session(replacement) = identity
        .login_with_password(&account.email, new_password.into(), TraceContext::default())
        .await?
    else {
        anyhow::bail!("unexpected MFA");
    };
    let actor = identity.authenticate_session(&replacement.token).await?;
    identity
        .deactivate_account(&actor, TraceContext::default())
        .await?;
    assert!(
        identity
            .authenticate_session(&replacement.token)
            .await
            .is_err()
    );
    assert!(
        identity
            .login_with_password(&account.email, new_password.into(), TraceContext::default())
            .await
            .is_err()
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn verification_resend_is_pending_only_and_replaces_the_previous_challenge()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    identity
        .register_account(
            "pending@example.test",
            TEST_PASSWORD.into(),
            TraceContext::default(),
        )
        .await?;
    let previous = challenge::Entity::find()
        .one(&database.connection)
        .await?
        .unwrap();
    identity
        .request_verification("pending@example.test", TraceContext::default())
        .await?;
    let current = challenge::Entity::find()
        .one(&database.connection)
        .await?
        .unwrap();
    assert_ne!(previous.token_hash, current.token_hash);
    assert_eq!(
        challenge::Entity::find()
            .count(&database.connection)
            .await?,
        1
    );
    let (verified, _) = database.verified_account(&identity).await?;
    let email = identity.get_account(&verified).await?.email;
    let deliveries = mail_delivery::Entity::find()
        .count(&database.connection)
        .await?;
    identity
        .request_verification(&email, TraceContext::default())
        .await?;
    identity
        .request_verification("unknown@example.test", TraceContext::default())
        .await?;
    assert_eq!(
        mail_delivery::Entity::find()
            .count(&database.connection)
            .await?,
        deliveries
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn client_rate_limits_are_shared_without_exhausting_other_clients() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let first = database.identity_service().await?;
    let second = database.identity_service().await?;
    for attempt in 0..60 {
        (if attempt % 2 == 0 { &first } else { &second })
            .limit_client_authentication("192.0.2.10")
            .await?;
    }
    assert!(matches!(
        second.limit_client_authentication("192.0.2.10").await,
        Err(ApplicationError::RateLimited)
    ));
    second.limit_client_authentication("192.0.2.11").await?;
    database.finish().await
}
