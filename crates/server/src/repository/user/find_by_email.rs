use entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use errors::errors::Errors;
use sea_orm::sea_query::{Expr, ExprTrait, Func};
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter};

use crate::utils::email::normalize_email;

/// Looks up whether a user exists by email.
///
/// # Role
/// Normalizes the email (`normalize_email`), then fetches a single record matched
/// on `lower(email)`, so the same account is found regardless of case or
/// whitespace differences. The lookup uses the functional unique index on
/// `lower(email)`, so it is index-backed.
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
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
