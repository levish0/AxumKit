use sea_orm_migration::prelude::*;

pub(super) fn created_at(column_name: impl IntoIden) -> ColumnDef {
    ColumnDef::new(column_name)
        .timestamp_with_time_zone()
        .not_null()
        .default(Expr::current_timestamp())
        .to_owned()
}

pub(super) fn uuid_primary_key(column_name: impl IntoIden) -> ColumnDef {
    ColumnDef::new(column_name)
        .uuid()
        .not_null()
        .primary_key()
        .to_owned()
}
