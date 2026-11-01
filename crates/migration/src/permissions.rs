use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};

pub async fn configure(database: &DatabaseConnection) -> anyhow::Result<()> {
    let runtime = std::env::var("MIGRATION_RUNTIME_ROLE").unwrap_or_default();
    let maintenance = std::env::var("MIGRATION_MAINTENANCE_ROLE").unwrap_or_default();
    if runtime.is_empty() && maintenance.is_empty() {
        return Ok(());
    }
    anyhow::ensure!(
        !runtime.is_empty() && !maintenance.is_empty(),
        "configure both migration grant roles"
    );
    let transaction = database.begin().await?;
    // PostgreSQL role and GRANT administration has no SeaORM/SeaQuery equivalent.
    transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT set_config('hirune.runtime_role', $1, true), set_config('hirune.maintenance_role', $2, true)",
        [runtime.into(), maintenance.into()])).await?;
    transaction
        .execute_unprepared(include_str!("runtime_grants.sql"))
        .await?;
    transaction.commit().await?;
    Ok(())
}
