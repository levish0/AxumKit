use axum::{Router, extract::DefaultBodyLimit, middleware};
use axumkit_config::ServerConfig;
use axumkit_dto::action_logs::ActionLogResponse;
use axumkit_server::api::routes::api_routes;
use axumkit_server::connection::{
    MeilisearchClient, create_http_client, establish_read_connection, establish_r2_connection,
    establish_redis_connection, establish_seaweedfs_connection, establish_write_connection,
};
use axumkit_server::eventstream::start_eventstream_subscriber;
use axumkit_server::middleware::anonymous_user::anonymous_user_middleware;
use axumkit_server::middleware::cors::cors_layer;
use axumkit_server::middleware::trace_layer_config::make_span_with_request_id;
use axumkit_server::state::AppState;
use axumkit_server::utils::logger::init_tracing;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_cookies::CookieManagerLayer;
use tower_http::LatencyUnit;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{Level, error};

pub async fn run_server() -> anyhow::Result<()> {
    let write_db = establish_write_connection().await;
    let read_db = establish_read_connection().await;
    let r2_client = establish_r2_connection().await.map_err(|e| {
        error!("Failed to establish cloudflare_r2 connection: {}", e);
        anyhow::anyhow!("R2 connection failed: {}", e)
    })?;
    let seaweedfs_client = establish_seaweedfs_connection().await.map_err(|e| {
        error!("Failed to establish SeaweedFS connection: {}", e);
        anyhow::anyhow!("SeaweedFS connection failed: {}", e)
    })?;
    let redis_session = establish_redis_connection(
        &ServerConfig::get().redis_session_host,
        &ServerConfig::get().redis_session_port,
        "Session",
    )
    .await
    .map_err(|e| {
        error!("Failed to establish redis session connection: {}", e);
        anyhow::anyhow!("Redis session connection failed: {}", e)
    })?;

    let redis_cache = establish_redis_connection(
        &ServerConfig::get().redis_cache_host,
        &ServerConfig::get().redis_cache_port,
        "Cache",
    )
    .await
    .map_err(|e| {
        error!("Failed to establish redis cache connection: {}", e);
        anyhow::anyhow!("Redis cache connection failed: {}", e)
    })?;

    // Connect to NATS and create JetStream context
    let nats_client = async_nats::connect(&ServerConfig::get().nats_url)
        .await
        .map_err(|e| {
            error!("Failed to connect to NATS: {}", e);
            anyhow::anyhow!("NATS connection failed: {}", e)
        })?;
    let worker = Arc::new(async_nats::jetstream::new(nats_client.clone()));

    // Create broadcast channel for EventStream SSE fan-out
    let (eventstream_tx, _) = broadcast::channel::<ActionLogResponse>(1000);

    // Start EventStream subscriber background task
    let subscriber_nats = nats_client.clone();
    let subscriber_tx = eventstream_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = start_eventstream_subscriber(subscriber_nats, subscriber_tx).await {
            error!("EventStream subscriber failed: {}", e);
        }
    });

    let http_client = create_http_client().await.map_err(|e| {
        error!("Failed to create HTTP client: {}", e);
        anyhow::anyhow!("HTTP client creation failed: {}", e)
    })?;
    let meilisearch_client = MeilisearchClient::new().map_err(|e| {
        error!("Failed to create Meilisearch client: {}", e);
        anyhow::anyhow!("Meilisearch client creation failed: {}", e)
    })?;

    let server_url = format!(
        "{}:{}",
        &ServerConfig::get().server_host,
        &ServerConfig::get().server_port
    );

    let state = AppState {
        write_db,
        read_db,
        r2_client,
        seaweedfs_client,
        redis_session,
        redis_cache,
        worker,
        nats_client,
        eventstream_tx,
        http_client,
        meilisearch_client,
    };

    let app = Router::new()
        .merge(api_routes(state.clone()))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024)) // 8MB default body limit
        .layer(middleware::from_fn(anonymous_user_middleware))
        .layer(CookieManagerLayer::new())
        .layer(cors_layer())
        // HTTP request/response tracing with request ID
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_span_with_request_id)
                .on_response(
                    tower_http::trace::DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .with_state(state);

    println!("Starting server at: {}", server_url);

    let listener = tokio::net::TcpListener::bind(&server_url).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    // tracing 초기화
    init_tracing();

    if let Err(err) = run_server().await {
        eprintln!("Application error: {}", err);
    }
}
