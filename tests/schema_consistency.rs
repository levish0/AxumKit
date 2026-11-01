#[path = "schema_consistency/mod.rs"]
mod schema_consistency;

#[path = "schema_consistency/migration_preflight.rs"]
mod migration_preflight;

use anyhow::{Result, ensure};
use schema_consistency::{catalog, enums};
use sea_orm::{ConnectionTrait, TransactionTrait};
use test_support::TestDatabase;

const ENTITY_SCHEMA: &str = "entity_reference";

async fn prepare_entity_schema(database: &TestDatabase) -> Result<()> {
    let transaction = database.connection.begin().await?;
    // SchemaBuilder keeps entity metadata private. Use PostgreSQL to describe its output.
    transaction
        .execute_unprepared("CREATE SCHEMA entity_reference")
        .await?;
    transaction
        .execute_unprepared("SET LOCAL search_path TO entity_reference")
        .await?;
    database
        .connection
        .get_schema_registry("application::*")
        .apply(&transaction)
        .await?;
    database
        .connection
        .get_schema_registry("jobs::*")
        .apply(&transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn migrations_match_registered_entities_and_enum_values() -> Result<()> {
    let database = TestDatabase::create().await?;
    let result = async {
        prepare_entity_schema(&database).await?;
        let problems = schema_consistency::inspect(&database.connection, ENTITY_SCHEMA).await?;
        ensure!(
            problems.is_empty(),
            "schema drift:\n{}",
            problems.join("\n")
        );
        Ok(())
    }
    .await;
    database.finish().await?;
    result
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn schema_guard_detects_structural_and_enum_drift() -> Result<()> {
    let database = TestDatabase::create().await?;
    let result = async {
        prepare_entity_schema(&database).await?;
        let baseline = schema_consistency::inspect(&database.connection, ENTITY_SCHEMA).await?;
        ensure!(
            baseline.is_empty(),
            "baseline schema drift:\n{}",
            baseline.join("\n")
        );
        let transaction = database.connection.begin().await?;
        transaction
            .execute_unprepared("ALTER TABLE public.posts DROP CONSTRAINT posts_state_check")
            .await?;
        transaction
            .execute_unprepared(
                "ALTER TABLE public.posts ADD CHECK (state IN ('deleted', 'draft', 'published'))",
            )
            .await?;
        let reordered = schema_consistency::inspect(&transaction, ENTITY_SCHEMA).await?;
        transaction.rollback().await?;
        ensure!(
            reordered.is_empty(),
            "enum value order must not cause schema drift: {reordered:#?}"
        );
        for case in schema_consistency::drift_cases::CASES {
            let transaction = database.connection.begin().await?;
            for statement in case.statements {
                transaction.execute_unprepared(statement).await?;
            }
            let problems = schema_consistency::inspect(&transaction, ENTITY_SCHEMA).await?;
            transaction.rollback().await?;
            let expected_problem = case.expected_problem;
            ensure!(
                problems
                    .iter()
                    .any(|problem| problem.contains(expected_problem)),
                "schema guard missed {expected_problem:?}; reported: {problems:#?}"
            );
        }
        let empty = std::collections::BTreeMap::new();
        let actual = catalog::read_schema(&database.connection, "public").await?;
        ensure!(
            catalog::compare_schemas(&empty, &actual)
                .iter()
                .any(|problem| problem == "entity registry is empty"),
            "an empty registry must fail validation"
        );
        let checks = catalog::read_checks(&database.connection).await?;
        let bindings = enums::bindings();
        let incomplete_bindings = &bindings[1..];
        ensure!(
            enums::compare_checks(incomplete_bindings, &checks)
                .iter()
                .any(|problem| problem.contains("unregistered enum CHECK")),
            "a missing enum binding must fail validation"
        );
        Ok(())
    }
    .await;
    database.finish().await?;
    result
}
