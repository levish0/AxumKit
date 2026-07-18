pub use sea_orm_migration::prelude::*;

mod common;
mod m20250825_033638_user_role_enum;
mod m20250825_033639_users;
mod m20250825_033640_user_roles;
mod m20250825_033641_user_bans;
mod m20250825_033642_actor_kind_enum;
mod m20250825_033643_actors;
mod m20250825_033645_oauth_providers;
mod m20250825_033646_oauth_connections;
mod m20251105_043639_notification_type_enum;
mod m20251215_034351_action_resource_type_enum;
mod m20251215_034352_moderation_resource_type_enum;
mod m20251215_034415_create_action_logs;
mod m20260328_141037_create_boards;
mod m20260328_141047_create_board_posts;
mod m20260328_141048_create_board_comments;
mod m20260328_141049_notification_target_kind_enum;
mod m20260328_141050_notification_events;
mod m20260328_141052_notification_deliveries;
mod m20260328_141056_notification_preferences;
mod m20260328_141058_notification_action_preferences;
mod m20260405_073559_create_moderation_logs;
mod m20260705_000000_create_auth_events;
mod m20260705_000100_create_known_devices;
pub(crate) mod m20260710_000101_acl_groups;
mod m20260710_000102_acl_group_members;
mod m20260710_000103_acl_group_permissions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250825_033638_user_role_enum::Migration),
            Box::new(m20250825_033639_users::Migration),
            Box::new(m20250825_033640_user_roles::Migration),
            Box::new(m20250825_033641_user_bans::Migration),
            Box::new(m20250825_033642_actor_kind_enum::Migration),
            Box::new(m20250825_033643_actors::Migration),
            Box::new(m20250825_033645_oauth_providers::Migration),
            Box::new(m20250825_033646_oauth_connections::Migration),
            Box::new(m20251215_034351_action_resource_type_enum::Migration),
            Box::new(m20251215_034352_moderation_resource_type_enum::Migration),
            Box::new(m20251215_034415_create_action_logs::Migration),
            Box::new(m20251105_043639_notification_type_enum::Migration),
            Box::new(m20260328_141037_create_boards::Migration),
            Box::new(m20260328_141047_create_board_posts::Migration),
            Box::new(m20260328_141048_create_board_comments::Migration),
            Box::new(m20260328_141049_notification_target_kind_enum::Migration),
            Box::new(m20260328_141050_notification_events::Migration),
            Box::new(m20260328_141052_notification_deliveries::Migration),
            Box::new(m20260328_141056_notification_preferences::Migration),
            Box::new(m20260328_141058_notification_action_preferences::Migration),
            Box::new(m20260405_073559_create_moderation_logs::Migration),
            Box::new(m20260705_000000_create_auth_events::Migration),
            Box::new(m20260705_000100_create_known_devices::Migration),
            Box::new(m20260710_000101_acl_groups::Migration),
            Box::new(m20260710_000102_acl_group_members::Migration),
            Box::new(m20260710_000103_acl_group_permissions::Migration),
        ]
    }
}
