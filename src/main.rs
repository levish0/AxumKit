use crate::api::v0::routes::routes::api_routes;
use crate::config::db_config::DbConfig;
use crate::middleware::cors::cors_layer;
use crate::state::AppState;
use crate::utils::logger::init_tracing;
use axum::Router;
use std::net::SocketAddr;
use tower_http::compression::CompressionLayer;
use tracing::{error, info};
use crate::connection::database::establish_connection;

mod api;
mod config;
mod connection;
mod dto;
mod entity;
mod middleware;
mod service;
mod state;
mod utils;

pub async fn run_server() -> anyhow::Result<()> {
    let conn = establish_connection().await;

    let server_url = format!(
        "{}:{}",
        &DbConfig::get().server_host,
        &DbConfig::get().server_port
    );
    let app = Router::new()
        .merge(api_routes())
        .layer(cors_layer())
        .layer(CompressionLayer::new())
        .with_state(AppState {
            conn
        });

    info!("Starting server at: {}", server_url);

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
        eprintln!("Application error: {}", err);
    }
}