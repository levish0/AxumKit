use AxumKit::api::v0::routes::routes::api_routes;
use AxumKit::config::db_config::DbConfig;
use AxumKit::connection::database_conn::establish_connection;
use AxumKit::connection::http_conn::create_http_client;
use AxumKit::connection::r2_conn::establish_r2_connection;
use AxumKit::connection::redis_conn::establish_redis_connection;
use AxumKit::middleware::anonymous_user::anonymous_user_middleware;
use AxumKit::middleware::cors::cors_layer;
use AxumKit::middleware::trace_layer_config::make_span_with_request_id;
use AxumKit::state::AppState;
use AxumKit::utils::logger::init_tracing;
use axum::{Router, middleware};
use std::net::SocketAddr;
use tower_cookies::CookieManagerLayer;
use tower_http::LatencyUnit;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::error;
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
        &DbConfig::get().server_host,
        &DbConfig::get().server_port
    );

    let state = AppState {
        conn,
        // r2_client,
        redis_client,
        http_client,
    };

    let app = Router::new()
        .merge(api_routes(state.clone()))
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
