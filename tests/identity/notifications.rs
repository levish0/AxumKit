use super::*;
use mail::{AuthenticationMail, MailPurpose};
use sea_orm::{ColumnTrait, QueryFilter};

async fn pending_messages(database: &TestDatabase) -> anyhow::Result<Vec<AuthenticationMail>> {
    let settings = test_support::identity_settings()?;
    mail_delivery::Entity::find()
        .filter(mail_delivery::Column::State.eq(application::identity::MailDeliveryState::Pending))
        .all(&database.connection)
        .await?
        .into_iter()
        .map(|delivery| {
            let plaintext = settings.secret_keys.decrypt(
                &format!("authentication mail v1:{}", delivery.id),
                delivery.ciphertext.as_deref().unwrap(),
            )?;
            Ok(serde_json::from_slice(&plaintext)?)
        })
        .collect()
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn verification_queues_welcome_once_and_password_changes_queue_tokenless_notifications()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    identity
        .register_account(
            "notifications@example.test",
            TEST_PASSWORD.into(),
            TraceContext::default(),
        )
        .await?;
    let messages = pending_messages(&database).await?;
    assert_eq!(messages.len(), 1);
    assert!(matches!(messages[0].purpose, MailPurpose::VerifyEmail));
    let action_url: url::Url = messages[0].action_url.parse()?;
    let token = action_url
        .fragment()
        .unwrap()
        .strip_prefix("token=")
        .unwrap();
    identity
        .verify_email(token, TraceContext::default())
        .await?;
    assert!(
        identity
            .verify_email(token, TraceContext::default())
            .await
            .is_err()
    );
    let messages = pending_messages(&database).await?;
    assert_eq!(
        messages
            .iter()
            .filter(|mail| matches!(mail.purpose, MailPurpose::Welcome))
            .count(),
        1
    );
    let LoginOutcome::Session(credentials) = identity
        .login_with_password(
            "notifications@example.test",
            TEST_PASSWORD.into(),
            TraceContext::default(),
        )
        .await?
    else {
        anyhow::bail!("unexpected MFA challenge");
    };
    let actor = identity.authenticate_session(&credentials.token).await?;
    assert!(
        identity
            .change_password(
                &actor,
                "wrong password".into(),
                "replacement notification password".into(),
                TraceContext::default()
            )
            .await
            .is_err()
    );
    assert!(
        !pending_messages(&database)
            .await?
            .iter()
            .any(|mail| matches!(mail.purpose, MailPurpose::PasswordChanged))
    );
    identity
        .change_password(
            &actor,
            TEST_PASSWORD.into(),
            "replacement notification password".into(),
            TraceContext::default(),
        )
        .await?;
    let reset_token = generate_secure_token();
    database
        .insert_challenge(
            actor.account_id(),
            application::identity::ChallengePurpose::ResetPassword,
            &reset_token,
        )
        .await?;
    identity
        .reset_password(
            &reset_token,
            "another notification password".into(),
            TraceContext::default(),
        )
        .await?;
    let messages = pending_messages(&database).await?;
    let notifications: Vec<_> = messages
        .iter()
        .filter(|mail| {
            matches!(
                mail.purpose,
                MailPurpose::Welcome | MailPurpose::PasswordChanged
            )
        })
        .collect();
    assert_eq!(notifications.len(), 3);
    for mail in notifications {
        assert_eq!(mail.action_url, "http://localhost:3000/");
    }
    let tokenless = mail_delivery::Entity::find()
        .filter(mail_delivery::Column::ChallengeHash.is_null())
        .count(&database.connection)
        .await?;
    assert_eq!(tokenless, 3);
    database.finish().await
}
