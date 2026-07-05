use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Private-tier authentication audit log (OWASP ASVS V16): every authentication decision —
        // login success/failure, logout, credential/2FA changes — recorded durably with the actor
        // IP + user-agent. Never publicly exposed; intended for a restricted role and a ~90-day
        // retention purge. No FK to users: the row must survive account deletion for forensics, and
        // failed logins have no user_id.
        manager
            .create_table(
                Table::create()
                    .table(AuthEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuthEvents::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(ColumnDef::new(AuthEvents::UserId).uuid().null())
                    .col(ColumnDef::new(AuthEvents::EventType).text().not_null())
                    .col(ColumnDef::new(AuthEvents::Ip).inet().null())
                    .col(ColumnDef::new(AuthEvents::UserAgent).text().null())
                    .col(ColumnDef::new(AuthEvents::Metadata).json_binary().null())
                    .col(
                        ColumnDef::new(AuthEvents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .to_owned(),
            )
            .await?;

        // Per-user timeline ("all auth events for user X", newest-first via UUIDv7 id).
        manager
            .create_index(
                Index::create()
                    .name("idx_auth_events_user_id")
                    .table(AuthEvents::Table)
                    .col(AuthEvents::UserId)
                    .col(AuthEvents::Id)
                    .to_owned(),
            )
            .await?;

        // Retention purge scans by age.
        manager
            .create_index(
                Index::create()
                    .name("idx_auth_events_created_at")
                    .table(AuthEvents::Table)
                    .col(AuthEvents::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // IP-based lookups (abuse investigation), SP-GiST for the INET type.
        manager
            .create_index(
                Index::create()
                    .name("idx_auth_events_ip")
                    .table(AuthEvents::Table)
                    .col(AuthEvents::Ip)
                    .index_type(IndexType::Custom(Alias::new("spgist").into()))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuthEvents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum AuthEvents {
    Table,
    Id,
    UserId,
    EventType,
    Ip,
    UserAgent,
    Metadata,
    CreatedAt,
}
