use dotenvy::dotenv;
use sea_orm_migration::prelude::*;
use std::env;

fn required_env(name: &str, legacy_name: &str) -> String {
    env::var(name)
        .or_else(|_| env::var(legacy_name))
        .unwrap_or_else(|_| panic!("{name} not set (legacy {legacy_name} also missing)"))
}

#[async_std::main]
async fn main() {
    dotenv().ok();

    let db_user = required_env("POSTGRES_USER", "POSTGRES_WRITE_USER");
    let db_password = required_env("POSTGRES_PASSWORD", "POSTGRES_WRITE_PASSWORD");
    let db_host = required_env("POSTGRES_HOST", "POSTGRES_WRITE_HOST");
    let db_port = required_env("POSTGRES_PORT", "POSTGRES_WRITE_PORT")
        .parse::<u16>()
        .expect("Invalid POSTGRES_PORT");
    let db_name = required_env("POSTGRES_NAME", "POSTGRES_WRITE_NAME");

    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_user, db_password, db_host, db_port, db_name
    );

    unsafe {
        env::set_var("DATABASE_URL", database_url);
    }

    cli::run_cli(migration::Migrator).await;
}
