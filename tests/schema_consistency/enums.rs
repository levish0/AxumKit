use super::catalog::CheckDefinition;
use application::{
    files::{FileState, entities::file},
    identity::{
        ChallengePurpose, MailDeliveryState,
        entities::{challenge, mail_delivery},
    },
    notifications::{
        PushDeliveryState, PushPlatform,
        entities::{push_delivery, push_device},
    },
    organizations::{MembershipRole, entities::membership},
    posts::{PostState, entities::post},
};
use jobs::{
    AttemptOutcome, JobState,
    entities::{job, job_attempt},
};
use sea_orm::{ActiveEnum, EntityTrait, Iden};
use std::collections::{BTreeMap, BTreeSet};

pub struct EnumColumn {
    table: String,
    column: String,
    values: BTreeSet<String>,
}

fn bind<E: EntityTrait, A: ActiveEnum<Value = String>>(entity: E, column: E::Column) -> EnumColumn {
    EnumColumn {
        table: entity.table_name().into(),
        column: column.to_string(),
        values: A::values().into_iter().collect(),
    }
}

pub fn bindings() -> Vec<EnumColumn> {
    vec![
        bind::<challenge::Entity, ChallengePurpose>(challenge::Entity, challenge::Column::Purpose),
        bind::<mail_delivery::Entity, MailDeliveryState>(
            mail_delivery::Entity,
            mail_delivery::Column::State,
        ),
        bind::<membership::Entity, MembershipRole>(membership::Entity, membership::Column::Role),
        bind::<file::Entity, FileState>(file::Entity, file::Column::State),
        bind::<post::Entity, PostState>(post::Entity, post::Column::State),
        bind::<push_device::Entity, PushPlatform>(
            push_device::Entity,
            push_device::Column::Platform,
        ),
        bind::<push_delivery::Entity, PushDeliveryState>(
            push_delivery::Entity,
            push_delivery::Column::State,
        ),
        bind::<job::Entity, JobState>(job::Entity, job::Column::State),
        bind::<job_attempt::Entity, AttemptOutcome>(
            job_attempt::Entity,
            job_attempt::Column::Outcome,
        ),
    ]
}

pub fn compare_checks(bindings: &[EnumColumn], checks: &[CheckDefinition]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut registered = BTreeMap::new();
    for binding in bindings {
        let key = (binding.table.clone(), binding.column.clone());
        if registered.insert(key, binding).is_some() {
            problems.push(format!(
                "{}.{}: duplicate enum binding",
                binding.table, binding.column
            ));
        }
    }
    let mut found = BTreeSet::new();
    for check in checks {
        let [column] = check.columns.as_slice() else {
            continue;
        };
        let key = (check.table.clone(), column.clone());
        let Some(values) = membership_values(&check.expression, column) else {
            if expression_structure(&check.expression)
                .is_some_and(|(structure, _)| structure.contains("anyarray["))
            {
                problems.push(format!(
                    "{}.{}: unsupported enum CHECK {}: {}",
                    check.table, column, check.name, check.expression
                ));
            }
            continue;
        };
        let Some(binding) = registered.get(&key) else {
            problems.push(format!(
                "{}.{}: unregistered enum CHECK {}",
                check.table, column, check.name
            ));
            continue;
        };
        if !found.insert(key) {
            problems.push(format!(
                "{}.{}: multiple enum CHECK constraints",
                check.table, column
            ));
        }
        if !check.validated {
            problems.push(format!(
                "{}.{}: enum CHECK is not validated",
                check.table, column
            ));
        }
        let rust_only = binding.values.difference(&values).collect::<Vec<_>>();
        let database_only = values.difference(&binding.values).collect::<Vec<_>>();
        if !rust_only.is_empty() || !database_only.is_empty() {
            problems.push(format!("{}.{}: enum values differ; Rust-only values {rust_only:?}; database-only values {database_only:?}", check.table, column));
        }
    }
    for (table, column) in registered.keys() {
        if !found.contains(&(table.clone(), column.clone())) {
            problems.push(format!("{table}.{column}: missing enum CHECK"));
        }
    }
    problems
}

fn membership_values(expression: &str, column: &str) -> Option<BTreeSet<String>> {
    let (structure, values) = expression_structure(expression)?;
    let (left, right) = structure.split_once('=')?;
    if left != column
        && left != format!("{column}::text")
        && left != format!("{column}::charactervarying")
    {
        return None;
    }
    let literal = |part: &str| matches!(part, "?" | "?::text" | "?::charactervarying");
    if literal(right) && values.len() == 1 {
        return Some(values.into_iter().collect());
    }
    let array = right.strip_prefix("anyarray[")?;
    let array = array
        .strip_suffix("]::text[]")
        .or_else(|| array.strip_suffix("]::charactervarying[]"))
        .or_else(|| array.strip_suffix(']'))?;
    let parts = array.split(',').collect::<Vec<_>>();
    if parts.len() != values.len() || !parts.into_iter().all(literal) || values.is_empty() {
        return None;
    }
    Some(values.into_iter().collect())
}

fn expression_structure(expression: &str) -> Option<(String, Vec<String>)> {
    let mut characters = expression.chars().peekable();
    let mut structure = String::new();
    let mut values = Vec::new();
    while let Some(character) = characters.next() {
        if character == '\'' {
            let mut value = String::new();
            loop {
                let character = characters.next()?;
                if character == '\'' {
                    if characters.peek() == Some(&'\'') {
                        characters.next();
                        value.push('\'');
                    } else {
                        break;
                    }
                } else {
                    value.push(character);
                }
            }
            values.push(value);
            structure.push('?');
        } else if !character.is_whitespace() && !matches!(character, '(' | ')' | '"') {
            structure.extend(character.to_lowercase());
        }
    }
    Some((structure, values))
}

#[cfg(test)]
mod tests {
    use super::membership_values;
    use std::collections::BTreeSet;

    #[test]
    fn recognizes_postgresql_membership_without_losing_literal_contents() {
        for expression in [
            "((state)::text = ANY ((ARRAY['draft'::character varying, 'published'::character varying])::text[]))",
            "state = ANY (ARRAY['draft'::text, 'published'::text])",
        ] {
            assert_eq!(
                membership_values(expression, "state"),
                Some(BTreeSet::from(["draft".into(), "published".into()]))
            );
        }
        assert_eq!(
            membership_values("state::text = 'a(b),''c'::text", "state"),
            Some(BTreeSet::from(["a(b),'c".into()]))
        );
    }

    #[test]
    fn rejects_compound_conditions_and_unsupported_expressions() {
        for expression in [
            "state = ANY (ARRAY['draft'::text]) OR true",
            "state = ANY (ARRAY['draft'::text]) AND state <> 'draft'",
            "state <> ALL (ARRAY['draft'::text])",
            "state = ANY (ARRAY[lower('DRAFT')])",
            "state = ANY (ARRAY[E'draft'])",
            "other = ANY (ARRAY['draft'::text])",
        ] {
            assert_eq!(
                membership_values(expression, "state"),
                None,
                "accepted unsupported CHECK: {expression}"
            );
        }
    }
}
