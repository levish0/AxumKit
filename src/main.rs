use std::net::SocketAddr;
use axum::Router;
use AxumKit::api::v0::routes::routes::api_routes;
use AxumKit::config::db_config::DbConfig;
use AxumKit::connection::database::establish_connection;
use AxumKit::middleware::cors::cors_layer;
use AxumKit::state::AppState;
use AxumKit::utils::logger::init_tracing;

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
        .with_state(AppState {
            conn
        });

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