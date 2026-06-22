use entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use errors::errors::Errors;
use sea_orm::sea_query::{Expr, ExprTrait, Func};
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter};

use crate::utils::email::normalize_email;

/// Get a user by email, case-insensitively (errors if absent). Normalizes the
/// email and matches against `lower(email)`, riding the functional unique index.
pub async fn repository_get_user_by_email<C>(conn: &C, email: String) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find()
        .filter(Expr::expr(Func::lower(Expr::col(UsersColumn::Email))).eq(normalize_email(&email)))
        .one(conn)
        .await?;

    user.ok_or(Errors::UserNotFound)
}
