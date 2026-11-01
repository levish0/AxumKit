use super::{catalog, enums};
use anyhow::Result;
use sea_orm::ConnectionTrait;

pub async fn inspect(database: &impl ConnectionTrait, entity_schema: &str) -> Result<Vec<String>> {
    let expected = catalog::read_schema(database, entity_schema).await?;
    let actual = catalog::read_schema(database, "public").await?;
    let mut problems = catalog::compare_schemas(&expected, &actual);
    let checks = catalog::read_checks(database).await?;
    problems.extend(enums::compare_checks(&enums::bindings(), &checks));
    problems.sort();
    Ok(problems)
}
