pub use sea_orm_migration::prelude::*;

mod common;
mod m20250825_033639_users;
mod m20250825_033642_create_posts;
mod m20250825_033645_oauth_providers;
mod m20250825_033646_oauth_connections;
mod m20251215_034351_action_resource_type_enum;
mod m20251215_034415_create_action_logs;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250825_033639_users::Migration),
            Box::new(m20250825_033642_create_posts::Migration),
            Box::new(m20250825_033645_oauth_providers::Migration),
            Box::new(m20250825_033646_oauth_connections::Migration),
            Box::new(m20251215_034351_action_resource_type_enum::Migration),
            Box::new(m20251215_034415_create_action_logs::Migration),
        ]
    }
}
