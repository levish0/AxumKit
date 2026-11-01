use super::*;
use sea_orm::{ColumnTrait, QueryFilter};

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn owner_management_is_scoped_and_new_clients_cannot_allocate_themselves_quota() -> Result<()>
{
    let fixture = Fixture::create(10, 100).await?;
    let (outsider, outsider_credentials) = fixture
        .database
        .verified_account(&fixture.app.identity)
        .await?;
    let foreign = fixture
        .app
        .organizations
        .create_organization(&outsider, "Other organization".into())
        .await?;
    let server = TestServer::new(kit::http::build_router(
        fixture.app.clone(),
        support::settings(&fixture.database)?.http,
    ));
    let owner = format!("Bearer {}", fixture.owner_token);
    let outsider_token = format!("Bearer {}", outsider_credentials.token);
    let base = format!(
        "/api/v1/organizations/{}/api-clients",
        fixture.organization_id
    );
    server
        .get(&base)
        .add_header(header::AUTHORIZATION, &outsider_token)
        .await
        .assert_status_not_found();
    server
        .get(&format!(
            "/api/v1/organizations/{}/api-clients/{}",
            foreign.id, fixture.client_id
        ))
        .add_header(header::AUTHORIZATION, &outsider_token)
        .await
        .assert_status_not_found();
    server.post(&base).add_header(header::AUTHORIZATION, &owner)
        .json(&json!({"name":"Cannot self-allocate","scopes":[ApiScope::OrganizationsRead],"month_limit":10000}))
        .await.assert_status_bad_request();
    let created = server
        .post(&base)
        .add_header(header::AUTHORIZATION, &owner)
        .json(&json!({"name":"Unprovisioned","scopes":[ApiScope::OrganizationsRead]}))
        .await;
    created.assert_status(StatusCode::CREATED);
    let client_id: Uuid = created.json::<Value>()["id"].as_str().unwrap().parse()?;
    assert_eq!(created.json::<Value>()["quota"]["month_limit"], 0);
    let issued = fixture
        .app
        .api_access
        .issue_key(
            &fixture.actor,
            fixture.organization_id,
            client_id,
            "Unprovisioned key".into(),
            1,
            TraceContext::default(),
        )
        .await?;
    assert!(matches!(
        fixture
            .app
            .api_access
            .read_organization(issued.credential.expose(), TraceContext::default())
            .await,
        Err(ApplicationError::Forbidden)
    ));
    server
        .put(&format!("{base}/{}", fixture.client_id))
        .add_header(header::AUTHORIZATION, &owner)
        .json(&json!({"name":"Restricted","scopes":[]}))
        .await
        .assert_status_ok();
    assert!(matches!(
        fixture
            .app
            .api_access
            .read_organization(&fixture.credential, TraceContext::default())
            .await,
        Err(ApplicationError::Forbidden)
    ));
    assert_eq!(
        api_usage::Entity::find()
            .count(&fixture.database.connection)
            .await?,
        0
    );
    let page = server
        .get(&format!("{base}?limit=1"))
        .add_header(header::AUTHORIZATION, &owner)
        .await;
    page.assert_status_ok();
    assert_eq!(page.json::<Value>()["items"].as_array().unwrap().len(), 1);
    assert!(page.json::<Value>()["next_cursor"].is_string());
    server
        .delete(&format!("{base}/{}", fixture.client_id))
        .add_header(header::AUTHORIZATION, &owner)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert!(matches!(
        fixture
            .app
            .api_access
            .read_organization(&fixture.credential, TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    fixture.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn key_management_requires_an_owner_and_recent_authentication_but_keys_outlive_the_creator()
-> Result<()> {
    use application::{identity::entities::session, organizations::entities::membership};
    let fixture = Fixture::create(10, 100).await?;
    let (member, _) = fixture
        .database
        .verified_account(&fixture.app.identity)
        .await?;
    membership::ActiveModel {
        id: Set(Uuid::now_v7()),
        organization_id: Set(fixture.organization_id),
        account_id: Set(member.account_id()),
        role: Set(application::organizations::MembershipRole::Member),
    }
    .insert(&fixture.database.connection)
    .await?;
    assert!(matches!(
        fixture
            .app
            .api_access
            .issue_key(
                &member,
                fixture.organization_id,
                fixture.client_id,
                "Denied".into(),
                1,
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Forbidden)
    ));
    let mut owner_session = session::Entity::find_by_id(fixture.actor.session_id())
        .one(&fixture.database.connection)
        .await?
        .unwrap()
        .into_active_model();
    owner_session.authenticated_at =
        Set(database_now(&fixture.database).await? - chrono::Duration::minutes(6));
    owner_session.update(&fixture.database.connection).await?;
    assert!(matches!(
        fixture
            .app
            .api_access
            .issue_key(
                &fixture.actor,
                fixture.organization_id,
                fixture.client_id,
                "Stale".into(),
                1,
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Forbidden)
    ));
    assert!(matches!(
        fixture
            .app
            .api_access
            .revoke_key(
                &fixture.actor,
                fixture.organization_id,
                fixture.client_id,
                fixture.key_id,
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Forbidden)
    ));
    fixture
        .app
        .api_access
        .get_client(&fixture.actor, fixture.organization_id, fixture.client_id)
        .await?;
    let mut owner_session = session::Entity::find_by_id(fixture.actor.session_id())
        .one(&fixture.database.connection)
        .await?
        .unwrap()
        .into_active_model();
    owner_session.authenticated_at = Set(database_now(&fixture.database).await?);
    owner_session.update(&fixture.database.connection).await?;
    fixture
        .app
        .identity
        .deactivate_account(&fixture.actor, TraceContext::default())
        .await?;
    fixture
        .app
        .api_access
        .read_organization(&fixture.credential, TraceContext::default())
        .await?;
    assert!(matches!(
        fixture
            .app
            .api_access
            .get_client(&fixture.actor, fixture.organization_id, fixture.client_id)
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    fixture.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn concurrent_creation_respects_metadata_caps_including_retained_revoked_records()
-> Result<()> {
    let fixture = Fixture::create(10, 100).await?;
    let now = database_now(&fixture.database).await?;
    let retained_keys = (0..98).map(|_| api_key::ActiveModel {
        id: Set(Uuid::now_v7()),
        client_id: Set(fixture.client_id),
        name: Set("Retained key".into()),
        secret_hash: Set("0".repeat(64)),
        expires_at: Set(now - chrono::Duration::days(1)),
        revoked_at: Set(Some(now)),
        ..Default::default()
    });
    api_key::Entity::insert_many(retained_keys)
        .exec(&fixture.database.connection)
        .await?;
    let retained_clients = (0..98).map(|_| api_client::ActiveModel {
        id: Set(Uuid::now_v7()),
        organization_id: Set(fixture.organization_id),
        name: Set("Retained client".into()),
        scopes: Set(json!([])),
        active: Set(false),
        ..Default::default()
    });
    api_client::Entity::insert_many(retained_clients)
        .exec(&fixture.database.connection)
        .await?;
    for creating_keys in [true, false] {
        let mut attempts = Vec::new();
        for _ in 0..2 {
            let service = fixture.app.api_access.clone();
            let actor = fixture.actor.clone();
            let organization_id = fixture.organization_id;
            let client_id = fixture.client_id;
            attempts.push(tokio::spawn(async move {
                if creating_keys {
                    service
                        .issue_key(
                            &actor,
                            organization_id,
                            client_id,
                            "Last slot".into(),
                            1,
                            TraceContext::default(),
                        )
                        .await
                        .map(|_| ())
                } else {
                    service
                        .create_client(
                            &actor,
                            organization_id,
                            "Last slot".into(),
                            vec![],
                            TraceContext::default(),
                        )
                        .await
                        .map(|_| ())
                }
            }));
        }
        let mut created = 0;
        let mut rejected = 0;
        for attempt in attempts {
            match attempt.await? {
                Ok(()) => created += 1,
                Err(ApplicationError::Conflict) => rejected += 1,
                _ => anyhow::bail!("unexpected metadata-cap outcome"),
            }
        }
        assert_eq!((created, rejected), (1, 1));
    }
    assert_eq!(
        api_key::Entity::find()
            .filter(api_key::Column::ClientId.eq(fixture.client_id))
            .count(&fixture.database.connection)
            .await?,
        100
    );
    assert_eq!(
        api_client::Entity::find()
            .filter(api_client::Column::OrganizationId.eq(fixture.organization_id))
            .count(&fixture.database.connection)
            .await?,
        100
    );
    fixture.finish().await
}
