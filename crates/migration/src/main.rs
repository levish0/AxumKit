mod permissions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let check = match arguments.as_slice() {
        [] => false,
        [argument] if argument == "--check-pending" => true,
        _ => anyhow::bail!("usage: migration [--check-pending]"),
    };
    let url = std::env::var("MIGRATION_DATABASE_URL").or_else(|_| std::env::var("DATABASE_URL"))?;
    let mut options = sea_orm::ConnectOptions::new(url);
    options.max_connections(1).sqlx_logging(false);
    let db = sea_orm::Database::connect(options).await?;
    if check {
        let pending = migration::pending_migration_count(&db).await?;
        let report = format!("{{\"pending_migrations\":{pending}}}");
        // Kubernetes consumes only this successful container's termination message.
        if let Ok(path) = std::env::var("MIGRATION_REPORT_PATH") {
            std::fs::write(path, &report)?;
        }
        println!("{report}");
    } else {
        migration::migrate(&db).await?;
        permissions::configure(&db).await?;
    }
    db.close().await?;
    Ok(())
}
