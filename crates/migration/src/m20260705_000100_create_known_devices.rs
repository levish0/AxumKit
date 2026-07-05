use crate::m20250825_033639_users::Users;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Trusted-device registry for new-device login verification (OWASP ASVS 6.3.5).
        // A row is created after a device is verified; on later logins a matching
        // (user_id, device_hash) skips the email challenge. `device_hash` is the blake3 hash of the
        // raw device-cookie token (never stored raw). FK-cascades on account deletion (not audit
        // data — safe to remove with the user).
        manager
            .create_table(
                Table::create()
                    .table(KnownDevices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(KnownDevices::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(KnownDevices::UserId).uuid().not_null())
                    .col(ColumnDef::new(KnownDevices::DeviceHash).text().not_null())
                    .col(ColumnDef::new(KnownDevices::UserAgent).text().null())
                    .col(ColumnDef::new(KnownDevices::LastIp).inet().null())
                    .col(
                        ColumnDef::new(KnownDevices::FirstSeen)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(KnownDevices::LastSeen)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_known_devices_user")
                            .from(KnownDevices::Table, KnownDevices::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // One row per (user, device); the lookup on login is by this pair.
        manager
            .create_index(
                Index::create()
                    .name("idx_known_devices_user_device")
                    .table(KnownDevices::Table)
                    .col(KnownDevices::UserId)
                    .col(KnownDevices::DeviceHash)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // List a user's devices.
        manager
            .create_index(
                Index::create()
                    .name("idx_known_devices_user_id")
                    .table(KnownDevices::Table)
                    .col(KnownDevices::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(KnownDevices::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum KnownDevices {
    Table,
    Id,
    UserId,
    DeviceHash,
    UserAgent,
    LastIp,
    FirstSeen,
    LastSeen,
}
