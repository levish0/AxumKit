use super::fixture::Fixture;
use application::{
    ApplicationError,
    files::{ReserveFile, entities::file},
    posts::{
        PostPublication, SavePost,
        entities::{post, post_attachment},
    },
};
use jobs::TraceContext;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, sea_query::Expr};
use uuid::Uuid;

async fn uploaded(f: &Fixture, organization_id: Uuid) -> anyhow::Result<Uuid> {
    let reserved = f
        .app
        .files
        .reserve_file(
            &f.owner,
            organization_id,
            ReserveFile {
                filename: "example.txt".into(),
                media_type: Some("text/plain".into()),
                byte_size: 7,
            },
            TraceContext::default(),
        )
        .await?;
    let transfer = f
        .app
        .files
        .create_upload_url(
            &f.owner,
            organization_id,
            reserved.id,
            TraceContext::default(),
        )
        .await?;
    let mut request =
        test_support::storage::request(reqwest::Method::PUT, &transfer.url)?.body("content");
    for header in &transfer.headers {
        request = request.header(&header.name, &header.value);
    }
    anyhow::ensure!(
        request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("direct PUT failed"))?
            .status()
            .is_success(),
        "direct PUT refused"
    );
    f.app
        .files
        .complete_upload(
            &f.owner,
            organization_id,
            reserved.id,
            TraceContext::default(),
        )
        .await?;
    Ok(reserved.id)
}

#[tokio::test]
#[ignore = "requires PostgreSQL, SeaweedFS, and Meilisearch; run just test-content"]
async fn attachments_reject_cross_organization_files_and_cleanup_cascades_only_links()
-> anyhow::Result<()> {
    let f = Fixture::new(true, false).await?;
    let second_org = f
        .app
        .organizations
        .create_organization(&f.owner, "Another scope".into())
        .await?;
    let foreign_file = uploaded(&f, second_org.id).await?;
    let result = f
        .app
        .posts
        .create_post(
            &f.owner,
            f.organization_id,
            SavePost {
                title: "Wrong scope".into(),
                body: "Must roll back".into(),
                state: PostPublication::Published,
                attachment_ids: vec![foreign_file],
                expected_version: 0,
            },
            TraceContext::default(),
        )
        .await;
    assert!(matches!(result, Err(ApplicationError::NotFound)));
    assert_eq!(post::Entity::find().count(&f.database.connection).await?, 0);
    let first = uploaded(&f, f.organization_id).await?;
    let second = uploaded(&f, f.organization_id).await?;
    let post = f
        .app
        .posts
        .create_post(
            &f.owner,
            f.organization_id,
            SavePost {
                title: "Attachments".into(),
                body: "Organization-readable originals".into(),
                state: PostPublication::Draft,
                attachment_ids: vec![first, second],
                expected_version: 0,
            },
            TraceContext::default(),
        )
        .await?;
    assert_eq!(post.id.get_version_num(), 7);
    f.app
        .files
        .delete_file(&f.owner, f.organization_id, first, TraceContext::default())
        .await?;
    f.app
        .files
        .delete_file(&f.owner, f.organization_id, second, TraceContext::default())
        .await?;
    assert!(
        f.app
            .posts
            .get_post(&f.owner, f.organization_id, post.id)
            .await?
            .attachment_ids
            .is_empty()
    );
    f.app
        .posts
        .delete_post(
            &f.owner,
            f.organization_id,
            post.id,
            1,
            TraceContext::default(),
        )
        .await?;
    file::Entity::update_many()
        .col_expr(
            file::Column::UploadExpiresAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::hours(2)),
        )
        .filter(file::Column::Id.is_in([first, second]))
        .exec(&f.database.connection)
        .await?;
    f.drain().await?;
    let now = chrono::Utc::now();
    file::Entity::update_many()
        .col_expr(
            file::Column::UpdatedAt,
            Expr::value(now - chrono::Duration::days(31)),
        )
        .col_expr(
            file::Column::UploadExpiresAt,
            Expr::value(now - chrono::Duration::days(31)),
        )
        .col_expr(file::Column::CleanupCompletedAt, Expr::value(Some(now)))
        .filter(file::Column::Id.is_in([first, second]))
        .exec(&f.database.connection)
        .await?;
    f.app.files.reconcile_files().await?;
    assert_eq!(
        file::Entity::find()
            .filter(file::Column::Id.is_in([first, second]))
            .count(&f.database.connection)
            .await?,
        0
    );
    assert_eq!(
        post_attachment::Entity::find()
            .filter(post_attachment::Column::PostId.eq(post.id))
            .count(&f.database.connection)
            .await?,
        0
    );
    assert!(
        post::Entity::find_by_id(post.id)
            .one(&f.database.connection)
            .await?
            .is_some()
    );
    assert!(
        file::Entity::find_by_id(foreign_file)
            .one(&f.database.connection)
            .await?
            .is_some()
    );
    f.finish().await
}
