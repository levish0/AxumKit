//! Health check E2E test

use e2e::TestServer;

#[tokio::test]
async fn test_health_check() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();

    // Start test app
    let app = TestServer::start()
        .await
        .expect("Failed to start TestServer");

    // Test health check endpoint
    let response = app
        .client
        .get("/health-check")
        .await
        .expect("Request failed");
    assert_eq!(response.status().as_u16(), 204);

    // Cleanup
    app.stop().await.expect("Failed to stop TestServer");
}
