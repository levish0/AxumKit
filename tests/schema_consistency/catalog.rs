use anyhow::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use std::collections::BTreeMap;

pub type SchemaDefinition = BTreeMap<String, BTreeMap<String, ColumnDefinition>>;

#[derive(Debug, PartialEq, Eq)]
pub struct ColumnDefinition {
    pub data_type: String,
    pub nullable: bool,
    pub primary_key_position: Option<i32>,
}

pub struct CheckDefinition {
    pub table: String,
    pub name: String,
    pub columns: Vec<String>,
    pub expression: String,
    pub validated: bool,
}

pub async fn read_schema(
    database: &impl ConnectionTrait,
    schema: &str,
) -> Result<SchemaDefinition> {
    // SeaQuery does not expose PostgreSQL catalog discovery. Schema names remain parameters.
    let rows = database
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"SELECT relation.relname AS table_name, attribute.attname AS column_name,
                      pg_catalog.format_type(attribute.atttypid, attribute.atttypmod) AS data_type,
                      NOT attribute.attnotnull AS nullable,
                      array_position(primary_key.conkey, attribute.attnum)::integer AS primary_key_position
               FROM pg_catalog.pg_class AS relation
               JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace
               LEFT JOIN pg_catalog.pg_attribute AS attribute
                 ON attribute.attrelid = relation.oid AND attribute.attnum > 0 AND NOT attribute.attisdropped
               LEFT JOIN pg_catalog.pg_constraint AS primary_key
                 ON primary_key.conrelid = relation.oid AND primary_key.contype = 'p'
               WHERE namespace.nspname = $1 AND relation.relkind IN ('r', 'p')
                 AND relation.relname <> 'seaql_migrations'
               ORDER BY relation.relname, attribute.attnum"#,
            [schema.into()],
        ))
        .await?;
    let mut tables = SchemaDefinition::new();
    for row in rows {
        let table: String = row.try_get("", "table_name")?;
        let columns = tables.entry(table).or_default();
        if let Some(column) = row.try_get::<Option<String>>("", "column_name")? {
            columns.insert(
                column,
                ColumnDefinition {
                    data_type: row.try_get("", "data_type")?,
                    nullable: row.try_get("", "nullable")?,
                    primary_key_position: row.try_get("", "primary_key_position")?,
                },
            );
        }
    }
    Ok(tables)
}

pub fn compare_schemas(expected: &SchemaDefinition, actual: &SchemaDefinition) -> Vec<String> {
    let mut problems = Vec::new();
    if expected.is_empty() {
        problems.push("entity registry is empty".into());
    }
    for (table, expected_columns) in expected {
        let Some(actual_columns) = actual.get(table) else {
            problems.push(format!("{table}: missing database table"));
            continue;
        };
        for (column, expected_column) in expected_columns {
            let Some(actual_column) = actual_columns.get(column) else {
                problems.push(format!("{table}.{column}: missing database column"));
                continue;
            };
            if expected_column.data_type != actual_column.data_type {
                problems.push(format!(
                    "{table}.{column}: type mismatch; Rust {}, database {}",
                    expected_column.data_type, actual_column.data_type
                ));
            }
            if expected_column.nullable != actual_column.nullable {
                problems.push(format!(
                    "{table}.{column}: nullable mismatch; Rust {}, database {}",
                    expected_column.nullable, actual_column.nullable
                ));
            }
            if expected_column.primary_key_position != actual_column.primary_key_position {
                problems.push(format!(
                    "{table}.{column}: primary key position mismatch; Rust {:?}, database {:?}",
                    expected_column.primary_key_position, actual_column.primary_key_position
                ));
            }
        }
        for column in actual_columns.keys() {
            if !expected_columns.contains_key(column) {
                problems.push(format!("{table}.{column}: missing entity column"));
            }
        }
    }
    for table in actual.keys() {
        if !expected.contains_key(table) {
            problems.push(format!("{table}: missing entity"));
        }
    }
    problems
}

pub async fn read_checks(database: &impl ConnectionTrait) -> Result<Vec<CheckDefinition>> {
    // pg_get_expr returns PostgreSQL's normalized expression, without the CHECK wrapper.
    let rows = database
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
        SELECT relation.relname AS table_name, constraint_row.conname AS constraint_name,
               constraint_row.convalidated AS validated,
               pg_catalog.pg_get_expr(constraint_row.conbin, constraint_row.conrelid) AS expression,
               (SELECT COALESCE(jsonb_agg(attribute.attname ORDER BY key.ordinality), '[]'::jsonb)
                FROM unnest(constraint_row.conkey) WITH ORDINALITY AS key(number, ordinality)
                JOIN pg_catalog.pg_attribute AS attribute
                  ON attribute.attrelid = relation.oid AND attribute.attnum = key.number) AS columns
        FROM pg_catalog.pg_constraint AS constraint_row
        JOIN pg_catalog.pg_class AS relation ON relation.oid = constraint_row.conrelid
        JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public' AND constraint_row.contype = 'c'
        ORDER BY relation.relname, constraint_row.conname
    "#,
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            Ok(CheckDefinition {
                table: row.try_get("", "table_name")?,
                name: row.try_get("", "constraint_name")?,
                columns: serde_json::from_value(row.try_get("", "columns")?)?,
                expression: row.try_get("", "expression")?,
                validated: row.try_get("", "validated")?,
            })
        })
        .collect()
}
