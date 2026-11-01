use anyhow::{Result, ensure};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use test_support::TestDatabase;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn migration_preflight_reads_fresh_and_current_history_and_rejects_corruption() -> Result<()>
{
    let database = TestDatabase::create().await?;
    let result = async {
        let db = &database.connection;
        ensure!(migration::pending_migration_count(db).await? == 0);
        let rows = db
            .query_all_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT version FROM seaql_migrations ORDER BY version".to_owned(),
            ))
            .await?;
        let first: String = rows.first().unwrap().try_get("", "version")?;
        let last: String = rows.last().unwrap().try_get("", "version")?;
        // Deliberately corrupt only this disposable database's migration history.
        let change = |sql: &str, version: &str| {
            Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, [version.into()])
        };
        db.execute_raw(change(
            "DELETE FROM seaql_migrations WHERE version = $1",
            &last,
        ))
        .await?;
        ensure!(migration::pending_migration_count(db).await? == 1);
        db.execute_raw(change("INSERT INTO seaql_migrations VALUES ($1, 1)", &last))
            .await?;
        db.execute_raw(change(
            "DELETE FROM seaql_migrations WHERE version = $1",
            &first,
        ))
        .await?;
        ensure!(
            migration::pending_migration_count(db).await.is_err(),
            "preflight accepted an applied successor of a missing migration"
        );
        db.execute_raw(change(
            "INSERT INTO seaql_migrations VALUES ($1, 1)",
            &first,
        ))
        .await?;
        db.execute_unprepared(
            "INSERT INTO seaql_migrations VALUES ('unknown_applied_migration', 1)",
        )
        .await?;
        ensure!(
            migration::pending_migration_count(db).await.is_err(),
            "preflight accepted a database newer than the image"
        );
        db.execute_unprepared("DROP TABLE seaql_migrations").await?;
        ensure!(migration::pending_migration_count(db).await? == rows.len());
        let exists = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT to_regclass('public.seaql_migrations') IS NULL AS absent".to_owned(),
            ))
            .await?
            .unwrap();
        ensure!(
            exists.try_get::<bool>("", "absent")?,
            "preflight created migration history on a fresh database"
        );
        Ok(())
    }
    .await;
    database.finish().await?;
    result
}
