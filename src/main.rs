mod api;
mod config;
mod database;
mod dto;
mod entity;
mod middleware;
mod service;
mod state;

use crate::api::routes::routes::api_routes;
use crate::config::db_config::DbConfig;
use crate::database::connection::establish_connection;
use crate::middleware::cors::cors_layer;
use crate::state::AppState;
use axum::Router;
use tracing::info;

pub async fn run_server() -> anyhow::Result<()> {
    let db_config = DbConfig::from_env();

    let conn = establish_connection().await;

    let server_url = format!("{}:{}", db_config.server_host, db_config.server_port);

    let app = Router::new()
        .merge(api_routes())
        .layer(cors_layer())
        .with_state(AppState { conn });

    info!("Starting server at: {}", server_url);

    let listener = tokio::net::TcpListener::bind(&server_url).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    {
        tracing_subscriber::fmt::init();
    }
    if let Err(err) = run_server().await {
        eprintln!("Application error: {}", err);
    }
}
