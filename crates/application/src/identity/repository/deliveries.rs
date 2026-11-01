use super::super::entities::mail_delivery;
use crate::identity::MailDeliveryState;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    QueryFilter, Set, sea_query::Expr,
};
use uuid::Uuid;

pub async fn insert_delivery(
    transaction: &DatabaseTransaction,
    delivery_id: Uuid,
    account_id: Uuid,
    challenge_hash: Option<String>,
    ciphertext: String,
    expires_at: DateTime<Utc>,
) -> Result<(), DbErr> {
    mail_delivery::ActiveModel {
        id: Set(delivery_id),
        account_id: Set(account_id),
        challenge_hash: Set(challenge_hash),
        ciphertext: Set(Some(ciphertext)),
        expires_at: Set(expires_at),
        ..Default::default()
    }
    .insert(transaction)
    .await?;
    Ok(())
}
pub async fn find_delivery(
    connection: &impl ConnectionTrait,
    delivery_id: Uuid,
) -> Result<Option<mail_delivery::Model>, DbErr> {
    mail_delivery::Entity::find_by_id(delivery_id)
        .one(connection)
        .await
}
pub async fn finish_delivery(
    transaction: &DatabaseTransaction,
    delivery_id: Uuid,
    accepted_at: Option<DateTime<Utc>>,
) -> Result<(), DbErr> {
    mail_delivery::Entity::update_many()
        .col_expr(
            mail_delivery::Column::Ciphertext,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            mail_delivery::Column::State,
            Expr::value(if accepted_at.is_some() {
                MailDeliveryState::Accepted
            } else {
                MailDeliveryState::Expired
            }),
        )
        .col_expr(mail_delivery::Column::AcceptedAt, Expr::value(accepted_at))
        .filter(mail_delivery::Column::Id.eq(delivery_id))
        .filter(mail_delivery::Column::State.eq(MailDeliveryState::Pending))
        .exec(transaction)
        .await?;
    Ok(())
}
