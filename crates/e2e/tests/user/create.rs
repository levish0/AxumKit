//! User creation E2E tests

use e2e::TestServer;

#[tokio::test]
async fn test_create_user() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();

    let app = TestServer::start()
        .await
        .expect("Failed to start TestServer");
    app.run_migration().await.expect("Failed to run migration");

    // Create regular user
    let user = app.create_user().await.expect("Failed to create user");
    assert!(!user.session_id.is_empty());
    assert!(!user.handle.is_empty());
    tracing::info!("Created user: {} ({})", user.handle, user.email);

    // Create another user
    let user2 = app
        .create_user()
        .await
        .expect("Failed to create second user");
    assert!(!user2.session_id.is_empty());
    assert_ne!(user.id, user2.id);
    tracing::info!("Created user2: {} ({})", user2.handle, user2.email);

    app.stop().await.expect("Failed to stop TestServer");
}
