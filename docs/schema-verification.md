# Schema consistency

Run `just test-db` to compare the migrated PostgreSQL schema with the current
SeaORM entities and string-enum contracts. CI runs this command. Failures report
all detected differences in English.

## Comparison

The test creates a temporary database through the existing test fixture and
applies the complete migration history. SeaORM's test-only `entity-registry`
feature collects entities from `application::*` and `jobs::*`.

SeaORM keeps the registry's column metadata private. The test therefore creates
an `entity_reference` schema in the same temporary database and materializes the
registered entities there. It does not copy definitions from the migrations.
PostgreSQL describes both schemas through its catalogs, so type names and aliases
use the same database representation.

The comparison checks:

- Tables and columns in both directions. An empty registry fails the check.
- Exact PostgreSQL column types, including string lengths and JSON versus JSONB.
- Column nullability.
- Primary key columns and their positions, including composite keys.

The test covers application tables in `public`. It excludes the framework-owned
`seaql_migrations` table. It does not compare defaults, indexes, unique keys,
foreign keys, or general business CHECK expressions. These remain migration and
behavior-test responsibilities.

## String enums

Register each constrained string-enum column in
`tests/schema_consistency/enums.rs`. Bind the entity and typed column to its Rust
`ActiveEnum`. The test reads values through `ActiveEnum::values()`; do not copy
the value list into the test.

The checker reads PostgreSQL's normalized CHECK expressions. It recognizes
standalone string membership in an array and single-value string equality, with
the supported text casts. It preserves quoted literal contents and requires the
whole expression to match. It does not extract all quoted strings from arbitrary
SQL. Unsupported single-column membership expressions fail the check.

Each binding needs one validated membership CHECK with exactly the Rust values.
Missing constraints, extra values, missing values, duplicate bindings, and
multiple membership constraints fail. A database membership CHECK without a
binding also fails.

The enum-to-column binding is explicit. A new enum column with neither a CHECK
nor a binding cannot be inferred from a PostgreSQL string type. Review both when
adding a persisted closed set. Cross-column business checks are outside this
membership comparison.

`ApiScope` also appears in JSON scope lists and the unconstrained
`api_usage.operation` column. These are not membership-CHECK bindings.

## Changes and failures

Keep each entity's column metadata accurate. Declare string lengths, text
columns, and JSONB explicitly. Preserve wire values when changing enum metadata.
Never make historical migrations depend on the current application enum.

Add schema changes as new migrations. Do not edit migrations that a consuming
project has already applied. If a mismatch is only missing entity metadata,
correct the entity to describe the existing schema.

The regression test deliberately changes test tables, columns, types, nullability,
primary keys, and enum constraints. Each change runs in a transaction that is
rolled back. It verifies that the checker reports the expected failure.

This check verifies the result of a fresh migration run. It does not prove that
data backfills, upgrades from a previous deployed version, rollback, or rolling
deployment are safe. Add those tests when a migration changes those contracts.
