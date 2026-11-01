use super::{NotificationService, NotificationSummary, entities::notification, repository};
use crate::{
    ApplicationError, ApplicationResult,
    database::database_time,
    identity::AuthenticatedSession,
    pagination::{ChronologicalPageRequest, ChronologicalPosition, Page},
};
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    TransactionTrait, sea_query::Expr,
};
use uuid::Uuid;

impl NotificationService {
    pub async fn list_notifications(
        &self,
        actor: &AuthenticatedSession,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<NotificationSummary>> {
        let scope = format!("notifications:{}", actor.account_id());
        let page = ChronologicalPageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let rows = repository::visible(actor.account_id())
            .filter(page.condition(
                (notification::Entity, notification::Column::CreatedAt),
                (notification::Entity, notification::Column::Id),
            ))
            .order_by_desc(notification::Column::CreatedAt)
            .order_by_desc(notification::Column::Id)
            .limit(page.limit + 1)
            .all(&transaction)
            .await?;
        let page = Page::from_chronological_rows(rows, page, &scope, |row| ChronologicalPosition {
            created_at: row.created_at,
            id: row.id,
        });
        transaction.commit().await?;
        Ok(Page {
            items: page.items.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        })
    }

    pub async fn get_notification(
        &self,
        actor: &AuthenticatedSession,
        notification_id: Uuid,
    ) -> ApplicationResult<NotificationSummary> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let row = repository::visible(actor.account_id())
            .filter(notification::Column::Id.eq(notification_id))
            .one(&transaction)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        transaction.commit().await?;
        Ok(row.into())
    }

    pub async fn unread_count(&self, actor: &AuthenticatedSession) -> ApplicationResult<u64> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let count = repository::visible(actor.account_id())
            .filter(notification::Column::ReadAt.is_null())
            .count(&transaction)
            .await?;
        transaction.commit().await?;
        Ok(count)
    }

    pub async fn mark_read(
        &self,
        actor: &AuthenticatedSession,
        notification_id: Uuid,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let row = repository::visible(actor.account_id())
            .filter(notification::Column::Id.eq(notification_id))
            .one(&transaction)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        if row.read_at.is_none() {
            notification::Entity::update_many()
                .col_expr(
                    notification::Column::ReadAt,
                    Expr::value(Some(database_time(&transaction).await?)),
                )
                .filter(notification::Column::Id.eq(notification_id))
                .filter(notification::Column::RecipientId.eq(actor.account_id()))
                .filter(notification::Column::ReadAt.is_null())
                .exec(&transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn mark_read_batch(&self, actor: &AuthenticatedSession) -> ApplicationResult<u64> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let selected = repository::visible(actor.account_id())
            .select_only()
            .column(notification::Column::Id)
            .filter(notification::Column::ReadAt.is_null())
            .order_by_asc(notification::Column::Id)
            .limit(1000)
            .into_query();
        let updated = notification::Entity::update_many()
            .col_expr(
                notification::Column::ReadAt,
                Expr::value(Some(database_time(&transaction).await?)),
            )
            .filter(notification::Column::Id.in_subquery(selected))
            .filter(notification::Column::RecipientId.eq(actor.account_id()))
            .filter(notification::Column::ReadAt.is_null())
            .exec(&transaction)
            .await?
            .rows_affected;
        transaction.commit().await?;
        Ok(updated)
    }
}
