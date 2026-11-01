use super::m20261013_000010_create_mail_deliveries::MailDeliveries;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MailDeliveries::Table)
                    .modify_column(
                        ColumnDef::new(MailDeliveries::ChallengeHash)
                            .string_len(64)
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Existing notification rows must be archived before restoring token-only deliveries.
        manager
            .alter_table(
                Table::alter()
                    .table(MailDeliveries::Table)
                    .modify_column(
                        ColumnDef::new(MailDeliveries::ChallengeHash)
                            .string_len(64)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }
}
