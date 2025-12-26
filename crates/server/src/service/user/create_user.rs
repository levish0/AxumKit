use crate::dto::user::{CreateUserRequest, CreateUserResponse};
use crate::errors::errors::{Errors, ServiceResult};
use crate::repository::user::{
    repository_create_user, repository_find_user_by_email, repository_find_user_by_handle,
};
use sea_orm::{DatabaseConnection, TransactionTrait};

pub async fn service_create_user(
    conn: &DatabaseConnection,
    payload: CreateUserRequest,
) -> ServiceResult<CreateUserResponse> {
    let txn = conn.begin().await?;

    // Check if user already exists by email
    let existing_user_by_email = repository_find_user_by_email(&txn, payload.email.clone()).await?;
    if existing_user_by_email.is_some() {
        return Err(Errors::UserEmailAlreadyExists);
    }

    // Check if user already exists by handle
    let existing_user_by_handle =
        repository_find_user_by_handle(&txn, payload.handle.clone()).await?;
    if existing_user_by_handle.is_some() {
        return Err(Errors::UserHandleAlreadyExists);
    }

    // Create user in database
    repository_create_user(
        &txn,
        payload.email,
        payload.handle,
        payload.display_name,
        payload.password,
    )
    .await?;

    txn.commit().await?;

    Ok(CreateUserResponse {
        message: "User created successfully".to_string(),
    })
}
