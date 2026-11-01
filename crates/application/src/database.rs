use chrono::{DateTime, Utc};
use sea_orm::{
    ConnectionTrait, DbErr,
    sea_query::{Func, Query},
};

pub async fn database_time(connection: &impl ConnectionTrait) -> Result<DateTime<Utc>, DbErr> {
    connection
        .query_one(
            &Query::select()
                .expr_as(Func::cust("clock_timestamp"), "database_time")
                .to_owned(),
        )
        .await?
        .ok_or_else(|| DbErr::Custom("database clock unavailable".into()))?
        .try_get("", "database_time")
}
