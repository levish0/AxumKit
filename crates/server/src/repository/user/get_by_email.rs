use entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use errors::errors::Errors;
use sea_orm::sea_query::{Expr, ExprTrait, Func};
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter};

use crate::utils::email::normalize_email;

/// Fetches a single user by email (errors if not found).
///
/// # Role
/// Normalizes the email, then queries a single row matched on `lower(email)`;
/// returns `Errors::UserNotFound` if no user exists.
///
/// # Errors
/// - `Errors::UserNotFound` if the user does not exist
/// - Returns a DB/repository error if the query fails.
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
