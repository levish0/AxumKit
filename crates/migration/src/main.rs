use dotenvy::dotenv;
use sea_orm_migration::prelude::*;

#[async_std::main]
async fn main() {
    // `run_cli` reads DATABASE_URL from the environment itself, so there is nothing to
    // assemble here — the deployment supplies the URL directly. In the Neon layout the
    // migration service is handed the DIRECT (non-pooler) endpoint: DDL runs long
    // transactions and belongs on a session-mode connection, not a transaction-mode pooler.
    dotenv().ok();

    cli::run_cli(migration::Migrator).await;
}
