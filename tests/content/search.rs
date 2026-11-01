use super::fixture::Fixture;
use application::{
    ApplicationError,
    identity::entities::account,
    posts::{PostPublication, SavePost},
    profiles::{SaveProfile, entities::profile},
};
use jobs::TraceContext;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Expr};
use serde_json::json;

#[tokio::test]
#[ignore = "requires PostgreSQL and Meilisearch; run just test-content"]
async fn search_pages_count_only_current_authorized_post_revisions() -> anyhow::Result<()> {
    let f = Fixture::new(true, false).await?;
    let one = f
        .post(&f.owner, "Juniper one", PostPublication::Published)
        .await?;
    let two = f
        .post(&f.writer, "Juniper two", PostPublication::Published)
        .await?;
    let draft = f
        .post(&f.owner, "Juniper draft", PostPublication::Draft)
        .await?;
    f.drain().await?;
    let first = f
        .app
        .posts
        .search_posts(&f.writer, f.organization_id, "Juniper", Some(1), Some(1))
        .await?;
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.total_in_window, 2);
    assert!(first.has_next_page);
    let second = f
        .app
        .posts
        .search_posts(&f.writer, f.organization_id, "Juniper", Some(2), Some(1))
        .await?;
    assert_eq!(second.items.len(), 1);
    assert_ne!(first.items[0].id, second.items[0].id);
    assert!(!second.has_next_page);
    let index = f.settings.post_search.as_ref().unwrap();
    f.insert_search_document(index,json!({"id":draft.id,"version":draft.version,"organization_id":f.organization_id,"title":"Juniper draft","body":"Never visible"})).await?;
    let outsider_org = f
        .app
        .organizations
        .create_organization(&f.outsider, "Other organization".into())
        .await?;
    let outsider_post = f
        .app
        .posts
        .create_post(
            &f.outsider,
            outsider_org.id,
            SavePost {
                title: "Juniper outsider".into(),
                body: "Private organization".into(),
                state: PostPublication::Published,
                attachment_ids: vec![],
                expected_version: 0,
            },
            TraceContext::default(),
        )
        .await?;
    f.insert_search_document(index,json!({"id":outsider_post.id,"version":1,"organization_id":f.organization_id,"title":"Juniper poisoned scope","body":"Other organization"})).await?;
    assert_eq!(
        f.app
            .posts
            .search_posts(&f.writer, f.organization_id, "Juniper", None, None)
            .await?
            .total_in_window,
        2
    );
    f.app
        .posts
        .update_post(
            &f.owner,
            f.organization_id,
            one.id,
            SavePost {
                title: "Cobalt".into(),
                body: "Current content".into(),
                state: PostPublication::Published,
                attachment_ids: vec![],
                expected_version: 1,
            },
            TraceContext::default(),
        )
        .await?;
    let stale = f
        .app
        .posts
        .search_posts(&f.writer, f.organization_id, "Juniper", None, None)
        .await?;
    assert_eq!(stale.total_in_window, 1);
    assert_eq!(stale.items[0].id, two.id);
    f.app
        .posts
        .delete_post(
            &f.writer,
            f.organization_id,
            two.id,
            1,
            TraceContext::default(),
        )
        .await?;
    assert_eq!(
        f.app
            .posts
            .search_posts(&f.writer, f.organization_id, "Juniper", None, None)
            .await?
            .total_in_window,
        0
    );
    assert!(matches!(
        f.app
            .posts
            .search_posts(&f.outsider, f.organization_id, "Juniper", None, None)
            .await,
        Err(ApplicationError::NotFound)
    ));
    assert!(matches!(
        f.app
            .posts
            .search_posts(&f.owner, f.organization_id, "Juniper", Some(11), Some(100))
            .await,
        Err(ApplicationError::InvalidInput(_))
    ));
    f.drain().await?;
    assert_eq!(
        f.app
            .posts
            .search_posts(&f.writer, f.organization_id, "Cobalt", None, None)
            .await?
            .total_in_window,
        1
    );
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and Meilisearch; run just test-content"]
async fn user_search_excludes_other_memberships_stale_names_and_inactive_accounts()
-> anyhow::Result<()> {
    let f = Fixture::new(true, false).await?;
    f.profile(&f.owner, "Juniper owner").await?;
    f.profile(&f.writer, "Juniper writer").await?;
    f.profile(&f.outsider, "Juniper outsider").await?;
    f.app
        .organizations
        .create_organization(&f.outsider, "Other organization".into())
        .await?;
    f.drain().await?;
    let result = f
        .app
        .profiles
        .search_users(&f.owner, f.organization_id, "Juniper", None, None)
        .await?;
    assert_eq!(result.total_in_window, 2);
    let outside_profile = profile::Entity::find()
        .filter(profile::Column::AccountId.eq(f.outsider.account_id()))
        .one(&f.database.connection)
        .await?
        .unwrap();
    f.insert_search_document(f.settings.profile_search.as_ref().unwrap(),json!({"id":outside_profile.id,"version":1,"organization_ids":[f.organization_id],"display_name":"Juniper outsider","bio":"Not a member"})).await?;
    assert_eq!(
        f.app
            .profiles
            .search_users(&f.owner, f.organization_id, "Juniper", None, None)
            .await?
            .total_in_window,
        2
    );
    f.app
        .profiles
        .save_profile(
            &f.owner,
            SaveProfile {
                display_name: "Cobalt owner".into(),
                bio: "Changed profile".into(),
                expected_version: 1,
            },
            TraceContext::default(),
        )
        .await?;
    let stale = f
        .app
        .profiles
        .search_users(&f.owner, f.organization_id, "Juniper", None, None)
        .await?;
    assert_eq!(stale.total_in_window, 1);
    assert_eq!(stale.items[0].account_id, f.writer.account_id());
    account::Entity::update_many()
        .col_expr(account::Column::Active, Expr::value(false))
        .filter(account::Column::Id.eq(f.writer.account_id()))
        .exec(&f.database.connection)
        .await?;
    assert_eq!(
        f.app
            .profiles
            .search_users(&f.owner, f.organization_id, "Juniper", None, None)
            .await?
            .total_in_window,
        0
    );
    f.drain().await?;
    assert_eq!(
        f.app
            .profiles
            .search_users(&f.owner, f.organization_id, "Cobalt", None, None)
            .await?
            .total_in_window,
        1
    );
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL and Meilisearch; run just test-content"]
async fn membership_changes_enqueue_user_projection_refresh_and_gate_current_results()
-> anyhow::Result<()> {
    let f = Fixture::new(true, false).await?;
    f.profile(&f.outsider, "Cobalt newcomer").await?;
    f.drain().await?;
    assert_eq!(
        f.app
            .profiles
            .search_users(&f.owner, f.organization_id, "Cobalt", None, None)
            .await?
            .total_in_window,
        0
    );
    f.app
        .organizations
        .add_member(&f.owner, f.organization_id, f.outsider.account_id())
        .await?;
    f.drain().await?;
    assert_eq!(
        f.app
            .profiles
            .search_users(&f.owner, f.organization_id, "Cobalt", None, None)
            .await?
            .total_in_window,
        1
    );
    f.app
        .organizations
        .remove_member(&f.owner, f.organization_id, f.outsider.account_id())
        .await?;
    assert_eq!(
        f.app
            .profiles
            .search_users(&f.owner, f.organization_id, "Cobalt", None, None)
            .await?
            .total_in_window,
        0
    );
    f.drain().await?;
    assert_eq!(
        f.app
            .profiles
            .search_users(&f.owner, f.organization_id, "Cobalt", None, None)
            .await?
            .total_in_window,
        0
    );
    f.finish().await
}
