pub use sea_orm_migration::prelude::*;
mod m20250515_163857_create_users_table;
mod m20250521_184105_user_refresh_tokens;
mod m20250531_054355_user_refresh_tokens_user_agent;


pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250515_163857_create_users_table::Migration),
            Box::new(m20250521_184105_user_refresh_tokens::Migration),
            Box::new(m20250531_054355_user_refresh_tokens_user_agent::Migration),
        ]
    }
}
