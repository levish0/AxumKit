use axumkit_entity::users::Model as UserModel;

/// OAuth sign-in result (for internal service logic)
#[derive(Debug)]
pub struct OAuthUserResult {
    /// User model
    pub user: UserModel,
    /// Whether the user was newly created
    pub is_new_user: bool,
}
