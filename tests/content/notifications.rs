use super::fixture::Fixture;
use application::{
    ApplicationError,
    notifications::{
        NotificationPreferences,
        entities::{notification, push_delivery, push_device},
    },
    posts::{PostPublication, SavePost, entities::comment},
};
use jobs::TraceContext;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    sea_query::{Alias, Table},
};

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn inbox_preferences_read_state_and_target_visibility_are_consistent() -> anyhow::Result<()> {
    let f = Fixture::new(false, false).await?;
    let post = f
        .post(&f.owner, "Notify", PostPublication::Published)
        .await?;
    let first = f
        .app
        .posts
        .create_comment(
            &f.writer,
            f.organization_id,
            post.id,
            "First reply".into(),
            TraceContext::default(),
        )
        .await?;
    f.app
        .posts
        .create_comment(
            &f.owner,
            f.organization_id,
            post.id,
            "Own reply".into(),
            TraceContext::default(),
        )
        .await?;
    let inbox = f
        .app
        .notifications
        .list_notifications(&f.owner, None, None)
        .await?;
    assert_eq!(inbox.items.len(), 1);
    let id = inbox.items[0].id;
    assert_eq!(inbox.items[0].comment_id, first.id);
    assert_eq!(id.get_version_num(), 7);
    assert_eq!(f.app.notifications.unread_count(&f.owner).await?, 1);
    assert!(matches!(
        f.app.notifications.mark_read(&f.writer, id).await,
        Err(ApplicationError::NotFound)
    ));
    f.app.notifications.mark_read(&f.owner, id).await?;
    f.app.notifications.mark_read(&f.owner, id).await?;
    assert_eq!(f.app.notifications.unread_count(&f.owner).await?, 0);
    f.app
        .notifications
        .save_preferences(
            &f.owner,
            NotificationPreferences {
                post_comments: false,
                push_enabled: true,
            },
        )
        .await?;
    f.app
        .posts
        .create_comment(
            &f.writer,
            f.organization_id,
            post.id,
            "Muted reply".into(),
            TraceContext::default(),
        )
        .await?;
    assert_eq!(
        f.app
            .notifications
            .list_notifications(&f.owner, None, None)
            .await?
            .items
            .len(),
        1
    );
    f.app
        .notifications
        .save_preferences(
            &f.owner,
            NotificationPreferences {
                post_comments: true,
                push_enabled: true,
            },
        )
        .await?;
    f.app
        .posts
        .create_comment(
            &f.writer,
            f.organization_id,
            post.id,
            "Enabled reply".into(),
            TraceContext::default(),
        )
        .await?;
    assert_eq!(f.app.notifications.unread_count(&f.owner).await?, 1);
    assert_eq!(
        push_delivery::Entity::find()
            .count(&f.database.connection)
            .await?,
        0
    );
    let draft = f
        .app
        .posts
        .update_post(
            &f.owner,
            f.organization_id,
            post.id,
            SavePost {
                title: "Hidden".into(),
                body: "Hidden".into(),
                state: PostPublication::Draft,
                attachment_ids: vec![],
                expected_version: 1,
            },
            TraceContext::default(),
        )
        .await?;
    assert_eq!(f.app.notifications.unread_count(&f.owner).await?, 0);
    assert!(
        f.app
            .notifications
            .list_notifications(&f.owner, None, None)
            .await?
            .items
            .is_empty()
    );
    assert!(matches!(
        f.app.notifications.get_notification(&f.owner, id).await,
        Err(ApplicationError::NotFound)
    ));
    f.app
        .posts
        .update_post(
            &f.owner,
            f.organization_id,
            post.id,
            SavePost {
                title: "Visible".into(),
                body: "Visible".into(),
                state: PostPublication::Published,
                attachment_ids: vec![],
                expected_version: draft.version,
            },
            TraceContext::default(),
        )
        .await?;
    f.app
        .posts
        .delete_comment(
            &f.writer,
            f.organization_id,
            post.id,
            first.id,
            1,
            TraceContext::default(),
        )
        .await?;
    assert_eq!(
        f.app
            .notifications
            .list_notifications(&f.owner, None, None)
            .await?
            .items
            .len(),
        1
    );
    assert_eq!(f.app.notifications.mark_read_batch(&f.owner).await?, 1);
    assert_eq!(f.app.notifications.mark_read_batch(&f.owner).await?, 0);
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn removed_members_cannot_read_old_notifications_or_counts() -> anyhow::Result<()> {
    let f = Fixture::new(false, false).await?;
    let post = f
        .post(&f.writer, "Member post", PostPublication::Published)
        .await?;
    f.app
        .posts
        .create_comment(
            &f.owner,
            f.organization_id,
            post.id,
            "Owner reply".into(),
            TraceContext::default(),
        )
        .await?;
    let id = f
        .app
        .notifications
        .list_notifications(&f.writer, None, None)
        .await?
        .items[0]
        .id;
    assert_eq!(f.app.notifications.unread_count(&f.writer).await?, 1);
    f.app
        .organizations
        .remove_member(&f.owner, f.organization_id, f.writer.account_id())
        .await?;
    assert_eq!(f.app.notifications.unread_count(&f.writer).await?, 0);
    assert!(
        f.app
            .notifications
            .list_notifications(&f.writer, None, None)
            .await?
            .items
            .is_empty()
    );
    assert!(matches!(
        f.app.notifications.mark_read(&f.writer, id).await,
        Err(ApplicationError::NotFound)
    ));
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn device_renewal_preserves_generation_but_account_transfer_skips_old_push()
-> anyhow::Result<()> {
    let f = Fixture::new(false, true).await?;
    let token = "test-only-fcm-device-token-with-no-real-provider";
    let device = f
        .app
        .notifications
        .register_device(
            &f.owner,
            token,
            application::notifications::PushPlatform::Web,
        )
        .await?;
    f.app
        .notifications
        .save_preferences(
            &f.owner,
            NotificationPreferences {
                post_comments: true,
                push_enabled: true,
            },
        )
        .await?;
    let post = f.post(&f.owner, "Push", PostPublication::Published).await?;
    f.app
        .posts
        .create_comment(
            &f.writer,
            f.organization_id,
            post.id,
            "Reply".into(),
            TraceContext::default(),
        )
        .await?;
    let delivery = push_delivery::Entity::find()
        .one(&f.database.connection)
        .await?
        .unwrap();
    assert_eq!(delivery.device_version, 1);
    assert_ne!(
        push_device::Entity::find_by_id(device.id)
            .one(&f.database.connection)
            .await?
            .unwrap()
            .encrypted_token,
        token
    );
    let renewed = f
        .app
        .notifications
        .register_device(
            &f.owner,
            token,
            application::notifications::PushPlatform::Web,
        )
        .await?;
    assert_eq!(device.id, renewed.id);
    assert_eq!(
        push_device::Entity::find_by_id(device.id)
            .one(&f.database.connection)
            .await?
            .unwrap()
            .version,
        delivery.device_version
    );
    f.app
        .notifications
        .register_device(
            &f.writer,
            token,
            application::notifications::PushPlatform::Web,
        )
        .await?;
    let transferred = push_device::Entity::find_by_id(device.id)
        .one(&f.database.connection)
        .await?
        .unwrap();
    assert_eq!(transferred.account_id, f.writer.account_id());
    assert_eq!(transferred.version, 2);
    let claim = f.app.queue.claim(30, 120).await?.unwrap();
    assert_eq!(claim.kind, application::notifications::FCM_DELIVERY_JOB);
    f.app
        .notifications
        .execute_job(&claim)
        .await
        .map_err(|failure| anyhow::anyhow!("{}", failure.code))?;
    f.app.queue.finish(&claim, None).await?;
    assert_eq!(
        push_delivery::Entity::find_by_id(delivery.id)
            .one(&f.database.connection)
            .await?
            .unwrap()
            .state,
        application::notifications::PushDeliveryState::Skipped
    );
    assert_eq!(f.app.notifications.list_devices(&f.writer).await?.len(), 1);
    assert!(f.app.notifications.list_devices(&f.owner).await?.is_empty());
    assert!(matches!(
        f.app.notifications.remove_device(&f.owner, device.id).await,
        Err(ApplicationError::NotFound)
    ));
    f.app
        .identity
        .revoke_session(
            &f.writer,
            f.writer_credentials.session_id,
            TraceContext::default(),
        )
        .await?;
    assert!(
        push_device::Entity::find_by_id(device.id)
            .one(&f.database.connection)
            .await?
            .is_none()
    );
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn a_read_notification_skips_push_without_loading_google_credentials() -> anyhow::Result<()> {
    let f = Fixture::new(false, true).await?;
    f.app
        .notifications
        .register_device(
            &f.owner,
            "test-only-fcm-token-for-read-cancellation",
            application::notifications::PushPlatform::Android,
        )
        .await?;
    f.app
        .notifications
        .save_preferences(
            &f.owner,
            NotificationPreferences {
                post_comments: true,
                push_enabled: true,
            },
        )
        .await?;
    let post = f
        .post(&f.owner, "Read before worker", PostPublication::Published)
        .await?;
    f.app
        .posts
        .create_comment(
            &f.writer,
            f.organization_id,
            post.id,
            "Reply".into(),
            TraceContext::default(),
        )
        .await?;
    let note = f
        .app
        .notifications
        .list_notifications(&f.owner, None, None)
        .await?
        .items[0]
        .id;
    f.app.notifications.mark_read(&f.owner, note).await?;
    f.drain().await?;
    assert_eq!(
        push_delivery::Entity::find()
            .one(&f.database.connection)
            .await?
            .unwrap()
            .state,
        application::notifications::PushDeliveryState::Skipped
    );
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn comment_inbox_and_push_registration_roll_back_when_enqueue_fails() -> anyhow::Result<()> {
    let f = Fixture::new(false, true).await?;
    f.app
        .notifications
        .register_device(
            &f.owner,
            "test-only-fcm-token-for-atomic-rollback",
            application::notifications::PushPlatform::Web,
        )
        .await?;
    f.app
        .notifications
        .save_preferences(
            &f.owner,
            NotificationPreferences {
                post_comments: true,
                push_enabled: true,
            },
        )
        .await?;
    let post = f
        .post(&f.owner, "Atomic comment", PostPublication::Published)
        .await?;
    f.database
        .connection
        .execute(
            &Table::rename()
                .table(Alias::new("jobs"), Alias::new("unavailable_jobs"))
                .to_owned(),
        )
        .await?;
    let result = f
        .app
        .posts
        .create_comment(
            &f.writer,
            f.organization_id,
            post.id,
            "Must roll back".into(),
            TraceContext::default(),
        )
        .await;
    f.database
        .connection
        .execute(
            &Table::rename()
                .table(Alias::new("unavailable_jobs"), Alias::new("jobs"))
                .to_owned(),
        )
        .await?;
    assert!(result.is_err());
    assert_eq!(
        comment::Entity::find()
            .filter(comment::Column::PostId.eq(post.id))
            .count(&f.database.connection)
            .await?,
        0
    );
    assert_eq!(
        notification::Entity::find()
            .count(&f.database.connection)
            .await?,
        0
    );
    assert_eq!(
        push_delivery::Entity::find()
            .count(&f.database.connection)
            .await?,
        0
    );
    f.finish().await
}
