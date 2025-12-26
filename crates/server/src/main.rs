use axum::extract::DefaultBodyLimit;
use axum::{Router, middleware};
use server::api::routes::api_routes;
use server::config::server_config::ServerConfig;
use server::connection::database_conn::establish_connection;
use server::connection::http_conn::create_http_client;
use server::connection::r2_conn::establish_r2_connection;
use server::connection::redis_conn::establish_redis_connection;
use server::middleware::anonymous_user::anonymous_user_middleware;
use server::middleware::cors::cors_layer;
use server::middleware::trace_layer_config::make_span_with_request_id;
use server::state::AppState;
use server::utils::logger::init_tracing;
use std::net::SocketAddr;
use tower_cookies::CookieManagerLayer;
use tower_http::LatencyUnit;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{Level, error};

pub async fn run_server() -> anyhow::Result<()> {
    let conn = establish_connection().await;
    let r2_client = establish_r2_connection().await.map_err(|e| {
        error!("Failed to establish cloudflare_r2 connection: {}", e);
        anyhow::anyhow!("R2 connection failed: {}", e)
    })?;
    let redis_client = establish_redis_connection().await.map_err(|e| {
        error!("Failed to establish redis connection: {}", e);
        anyhow::anyhow!("Redis connection failed: {}", e)
    })?;
    let http_client = create_http_client().await.map_err(|e| {
        error!("Failed to create HTTP client: {}", e);
        anyhow::anyhow!("HTTP client creation failed: {}", e)
    })?;

    let server_url = format!(
        "{}:{}",
        &ServerConfig::get().server_host,
        &ServerConfig::get().server_port
    );

    let state = AppState {
        conn,
        // r2_client,
        redis_client,
        http_client,
    };

    let app = Router::new()
        .merge(api_routes(state.clone()))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024)) // 8MB default body limit
        .layer(middleware::from_fn(anonymous_user_middleware))
        .layer(CookieManagerLayer::new())
        .layer(cors_layer())
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
    // tracing 초기화
    init_tracing();

    if let Err(err) = run_server().await {
        eprintln!("Application errors: {}", err);
    }
}
