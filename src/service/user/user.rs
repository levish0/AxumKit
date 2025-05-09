use crate::dto::user_dto::CreateUser;
use crate::service::error::errors::Errors;
use anyhow::Result;
use sea_orm::DatabaseConnection;

pub async fn create_user(conn: &DatabaseConnection, payload: CreateUser) -> Result<(), Errors> {
    println!(
        "Creating user with username: {}, password: {}, and email: {}",
        payload.username, payload.password, payload.email,
    );
    Ok(())
}
