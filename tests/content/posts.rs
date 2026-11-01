use super::fixture::Fixture;
use application::{
    ApplicationError,
    posts::{PostPublication, SavePost, entities::post},
};
use jobs::TraceContext;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Expr};

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn content_roles_drafts_versions_and_membership_are_enforced() -> anyhow::Result<()> {
    let f = Fixture::new(false, false).await?;
    let draft = f
        .post(&f.writer, "Private draft", PostPublication::Draft)
        .await?;
    assert!(
        f.app
            .posts
            .get_post(&f.writer, f.organization_id, draft.id)
            .await
            .is_ok()
    );
    assert!(
        f.app
            .posts
            .get_post(&f.owner, f.organization_id, draft.id)
            .await
            .is_ok()
    );
    assert!(matches!(
        f.app
            .posts
            .get_post(&f.outsider, f.organization_id, draft.id)
            .await,
        Err(ApplicationError::NotFound)
    ));
    let member_post = f
        .post(&f.owner, "Owner draft", PostPublication::Draft)
        .await?;
    assert!(matches!(
        f.app
            .posts
            .get_post(&f.writer, f.organization_id, member_post.id)
            .await,
        Err(ApplicationError::NotFound)
    ));
    assert!(
        f.app
            .posts
            .list_posts(
                &f.writer,
                f.organization_id,
                PostPublication::Draft,
                None,
                None
            )
            .await?
            .items
            .iter()
            .all(|post| post.author_id == f.writer.account_id())
    );
    assert!(matches!(
        f.app
            .posts
            .create_comment(
                &f.writer,
                f.organization_id,
                draft.id,
                "Not published".into(),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Forbidden)
    ));
    let published = f
        .app
        .posts
        .update_post(
            &f.writer,
            f.organization_id,
            draft.id,
            SavePost {
                title: "Published".into(),
                body: "Current text".into(),
                state: PostPublication::Published,
                attachment_ids: vec![],
                expected_version: draft.version,
            },
            TraceContext::default(),
        )
        .await?;
    assert_eq!(published.version, 2);
    assert!(matches!(
        f.app
            .posts
            .update_post(
                &f.writer,
                f.organization_id,
                draft.id,
                SavePost {
                    title: "Stale".into(),
                    body: "Stale".into(),
                    state: PostPublication::Published,
                    attachment_ids: vec![],
                    expected_version: 1
                },
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Conflict)
    ));
    let owner_post = f
        .post(&f.owner, "Published owner post", PostPublication::Published)
        .await?;
    assert!(matches!(
        f.app
            .posts
            .update_post(
                &f.writer,
                f.organization_id,
                owner_post.id,
                SavePost {
                    title: "Hijacked".into(),
                    body: "No".into(),
                    state: PostPublication::Published,
                    attachment_ids: vec![],
                    expected_version: 1
                },
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Forbidden)
    ));
    f.app
        .organizations
        .remove_member(&f.owner, f.organization_id, f.writer.account_id())
        .await?;
    assert!(matches!(
        f.app
            .posts
            .get_post(&f.writer, f.organization_id, draft.id)
            .await,
        Err(ApplicationError::NotFound)
    ));
    assert!(matches!(
        f.app
            .posts
            .list_comments(&f.writer, f.organization_id, draft.id, None, None)
            .await,
        Err(ApplicationError::NotFound)
    ));
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn chronological_cursor_handles_timestamp_ties_and_scope_changes() -> anyhow::Result<()> {
    let f = Fixture::new(false, false).await?;
    let first = f
        .post(&f.writer, "First", PostPublication::Published)
        .await?;
    let second = f
        .post(&f.writer, "Second", PostPublication::Published)
        .await?;
    let timestamp = chrono::Utc::now();
    post::Entity::update_many()
        .col_expr(post::Column::CreatedAt, Expr::value(timestamp))
        .filter(post::Column::Id.is_in([first.id, second.id]))
        .exec(&f.database.connection)
        .await?;
    let one = f
        .app
        .posts
        .list_posts(
            &f.writer,
            f.organization_id,
            PostPublication::Published,
            Some(1),
            None,
        )
        .await?;
    assert_eq!(one.items.len(), 1);
    assert_eq!(one.items[0].id, std::cmp::max(first.id, second.id));
    let two = f
        .app
        .posts
        .list_posts(
            &f.writer,
            f.organization_id,
            PostPublication::Published,
            Some(1),
            one.next_cursor.as_deref(),
        )
        .await?;
    assert_eq!(two.items[0].id, std::cmp::min(first.id, second.id));
    assert!(two.next_cursor.is_none());
    assert!(matches!(
        f.app
            .posts
            .list_posts(
                &f.writer,
                f.organization_id,
                PostPublication::Draft,
                Some(1),
                one.next_cursor.as_deref()
            )
            .await,
        Err(ApplicationError::InvalidInput(_))
    ));
    assert!(matches!(
        f.app
            .posts
            .list_posts(
                &f.owner,
                f.organization_id,
                PostPublication::Published,
                Some(1),
                one.next_cursor.as_deref()
            )
            .await,
        Err(ApplicationError::InvalidInput(_))
    ));
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn concurrent_edits_and_delete_comment_races_preserve_authority() -> anyhow::Result<()> {
    let f = Fixture::new(false, false).await?;
    let post = f
        .post(&f.owner, "Concurrent", PostPublication::Published)
        .await?;
    let update = || SavePost {
        title: "Edited".into(),
        body: "Text".into(),
        state: PostPublication::Published,
        attachment_ids: vec![],
        expected_version: 1,
    };
    let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            f.app.posts.update_post(
                &f.owner,
                f.organization_id,
                post.id,
                update(),
                TraceContext::default()
            ),
            f.app.posts.update_post(
                &f.owner,
                f.organization_id,
                post.id,
                update(),
                TraceContext::default()
            )
        )
    })
    .await?;
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(
        matches!(first, Err(ApplicationError::Conflict))
            || matches!(second, Err(ApplicationError::Conflict))
    );
    let (deletion, comment) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            f.app.posts.delete_post(
                &f.owner,
                f.organization_id,
                post.id,
                2,
                TraceContext::default()
            ),
            f.app.posts.create_comment(
                &f.writer,
                f.organization_id,
                post.id,
                "Concurrent reply".into(),
                TraceContext::default()
            )
        )
    })
    .await?;
    deletion?;
    assert!(comment.is_ok() || matches!(comment, Err(ApplicationError::NotFound)));
    assert_eq!(f.app.notifications.unread_count(&f.owner).await?, 0);
    assert!(
        f.app
            .notifications
            .list_notifications(&f.owner, None, None)
            .await?
            .items
            .is_empty()
    );
    f.finish().await
}
