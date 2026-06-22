use entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use errors::errors::Errors;
use sea_orm::sea_query::{Expr, ExprTrait, Func};
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter};

use crate::utils::email::normalize_email;

/// Find a user by email, case-insensitively. The email is normalized and matched
/// against `lower(email)`, so case/whitespace variants resolve to the same account
/// and the query rides the `users_email_lower_key` functional unique index.
pub async fn repository_find_user_by_email<C>(
    conn: &C,
    email: String,
) -> Result<Option<UserModel>, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find()
        .filter(Expr::expr(Func::lower(Expr::col(UsersColumn::Email))).eq(normalize_email(&email)))
        .one(conn)
        .await?;

    Ok(user)
}
