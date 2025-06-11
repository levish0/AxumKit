use dotenvy::dotenv;
use sea_orm_migration::prelude::*;
use std::env;

#[async_std::main]
async fn main() {
    dotenv().ok();

    let db_user = env::var("POSTGRES_USER").expect("POSTGRES_USER not set");
    let db_password = env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD not set");
    let db_host = env::var("POSTGRES_HOST").expect("POSTGRES_HOST not set");
    let db_port = env::var("POSTGRES_PORT")
        .expect("POSTGRES_PORT not set")
        .parse::<u16>()
        .expect("Invalid POSTGRES_PORT");
    let db_name = env::var("POSTGRES_NAME").expect("POSTGRES_NAME not set");

    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_user, db_password, db_host, db_port, db_name
    );

    unsafe {
        env::set_var("DATABASE_URL", database_url);
    }

    cli::run_cli(migration::Migrator).await;
}
