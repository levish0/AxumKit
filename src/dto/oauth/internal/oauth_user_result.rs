use crate::entity::users::Model as UserModel;

/// OAuth 로그인 결과 (내부 서비스 로직용)
#[derive(Debug)]
pub struct OAuthUserResult {
    /// User model
    pub user: UserModel,
    /// 신규 생성된 유저인지 여부
    pub is_new_user: bool,
}
