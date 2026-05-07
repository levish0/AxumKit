use entity::common::OAuthProvider;
use entity::user_oauth_connections::Column as OAuthColumn;
use entity::users::{Entity as UserEntity, Model as UserModel, Relation as UserRelation};
use errors::errors::Errors;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter, QuerySelect, RelationTrait,
};

/// Find a user by OAuth provider and provider_user_id
pub async fn repository_find_user_by_oauth<C>(
    conn: &C,
    provider: OAuthProvider,
    provider_user_id: &str,
) -> Result<Option<UserModel>, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find()
        .join(JoinType::InnerJoin, UserRelation::OAuthConnections.def())
        .filter(OAuthColumn::Provider.eq(provider))
        .filter(OAuthColumn::ProviderUserId.eq(provider_user_id))
        .one(conn)
        .await?;

    Ok(user)
}
