use super::{Action, Authorization, Principal};
use crate::{ApplicationError, ApplicationResult};
use cedar_policy::{Authorizer, Context, Decision, Entities, EntityUid, Request};
use serde_json::{Value, json};
use uuid::Uuid;

pub(crate) struct PostFacts {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_owner_id: Uuid,
    pub author_id: Uuid,
    pub published: bool,
    pub deleted: bool,
    pub organization_member: bool,
}

impl PostFacts {
    fn entity(&self) -> Value {
        json!({"uid":{"type":"Post","id":self.id.to_string()},"attrs":{
            "owner":{"__entity":{"type":"User","id":self.author_id.to_string()}},
            "organization_owner":{"__entity":{"type":"User","id":self.organization_owner_id.to_string()}},
            "organization_id":self.organization_id.to_string(),"published":self.published,"deleted":self.deleted
        },"parents":[]})
    }
}

impl Authorization {
    pub(crate) fn require_post(
        &self,
        principal: Principal,
        action: Action,
        post: PostFacts,
    ) -> ApplicationResult<()> {
        self.evaluate_content(
            ("User", principal.account_id),
            action.as_str(),
            ("Post", post.id),
            json!([user(principal), post.entity()]),
            json!({"organization_member":post.organization_member}),
        )
    }

    pub(crate) fn require_external_post(
        &self,
        client_id: Uuid,
        organization_id: Uuid,
        post: PostFacts,
    ) -> ApplicationResult<()> {
        self.evaluate_content(("ApiClient",client_id),"read_external_post",("Post",post.id),
            json!([{"uid":{"type":"ApiClient","id":client_id.to_string()},"attrs":{"active":true,"organization_id":organization_id.to_string()},"parents":[]},post.entity()]),json!({}))
    }

    pub(crate) fn require_delete_comment(
        &self,
        principal: Principal,
        comment_id: Uuid,
        author_id: Uuid,
        deleted: bool,
        post: PostFacts,
    ) -> ApplicationResult<()> {
        self.evaluate_content(("User",principal.account_id),"delete_comment",("Comment",comment_id),json!([
            user(principal),{"uid":{"type":"Comment","id":comment_id.to_string()},"attrs":{
                "owner":{"__entity":{"type":"User","id":author_id.to_string()}},
                "post_owner":{"__entity":{"type":"User","id":post.author_id.to_string()}},
                "organization_owner":{"__entity":{"type":"User","id":post.organization_owner_id.to_string()}},
                "parent_published":post.published && !post.deleted,"deleted":deleted
            },"parents":[]}
        ]),json!({"organization_member":post.organization_member}))
    }

    fn evaluate_content(
        &self,
        principal: (&str, Uuid),
        action: &str,
        resource: (&str, Uuid),
        entities: Value,
        context: Value,
    ) -> ApplicationResult<()> {
        let principal: EntityUid = format!("{}::\"{}\"", principal.0, principal.1)
            .parse()
            .map_err(ApplicationError::internal)?;
        let action: EntityUid = format!("Action::\"{action}\"")
            .parse()
            .map_err(ApplicationError::internal)?;
        let resource: EntityUid = format!("{}::\"{}\"", resource.0, resource.1)
            .parse()
            .map_err(ApplicationError::internal)?;
        let entities = Entities::from_json_value(entities, Some(&self.schema))
            .map_err(ApplicationError::internal)?;
        let context = Context::from_json_value(context, Some((&self.schema, &action)))
            .map_err(ApplicationError::internal)?;
        let request = Request::new(principal, action, resource, context, Some(&self.schema))
            .map_err(ApplicationError::internal)?;
        let response = Authorizer::new().is_authorized(&request, &self.policies, &entities);
        if response.diagnostics().errors().next().is_some() {
            return Err(ApplicationError::internal(anyhow::anyhow!(
                "authorization evaluation failed"
            )));
        }
        match response.decision() {
            Decision::Allow => Ok(()),
            Decision::Deny => Err(ApplicationError::Forbidden),
        }
    }
}

fn user(principal: Principal) -> Value {
    json!({"uid":{"type":"User","id":principal.account_id.to_string()},"attrs":{
        "active":principal.active,"verified":principal.verified,"administrator":principal.administrator
    },"parents":[]})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_and_comment_rules_require_current_membership_and_resource_authority() {
        let authorization = Authorization::new().unwrap();
        let organization_owner_id = Uuid::now_v7();
        let author_id = Uuid::now_v7();
        let member_id = Uuid::now_v7();
        let commenter_id = Uuid::now_v7();
        let principal = |account_id| Principal {
            account_id,
            active: true,
            verified: true,
            administrator: false,
        };
        let facts = |published, deleted, organization_member| PostFacts {
            id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            organization_owner_id,
            author_id,
            published,
            deleted,
            organization_member,
        };
        for account_id in [organization_owner_id, author_id, member_id] {
            assert!(
                authorization
                    .require_post(
                        principal(account_id),
                        Action::ReadPost,
                        facts(true, false, true),
                    )
                    .is_ok()
            );
            assert!(matches!(
                authorization.require_post(
                    principal(account_id),
                    Action::ReadPost,
                    facts(true, false, false),
                ),
                Err(ApplicationError::Forbidden)
            ));
            assert!(matches!(
                authorization.require_post(
                    principal(account_id),
                    Action::ReadPost,
                    facts(true, true, true),
                ),
                Err(ApplicationError::Forbidden)
            ));
        }
        for account_id in [organization_owner_id, author_id] {
            assert!(
                authorization
                    .require_post(
                        principal(account_id),
                        Action::ReadPost,
                        facts(false, false, true),
                    )
                    .is_ok()
            );
            for action in [Action::EditPost, Action::DeletePost] {
                assert!(
                    authorization
                        .require_post(principal(account_id), action, facts(true, false, true),)
                        .is_ok()
                );
            }
        }
        for action in [
            Action::ReadPost,
            Action::EditPost,
            Action::DeletePost,
            Action::CreateComment,
        ] {
            assert!(matches!(
                authorization
                    .require_post(principal(member_id), action, facts(false, false, true),),
                Err(ApplicationError::Forbidden)
            ));
        }
        assert!(
            authorization
                .require_post(
                    principal(member_id),
                    Action::CreateComment,
                    facts(true, false, true),
                )
                .is_ok()
        );
        for principal in [
            Principal {
                active: false,
                ..principal(author_id)
            },
            Principal {
                verified: false,
                ..principal(author_id)
            },
            Principal {
                administrator: true,
                ..principal(member_id)
            },
        ] {
            assert!(matches!(
                authorization.require_post(principal, Action::EditPost, facts(true, false, true),),
                Err(ApplicationError::Forbidden)
            ));
        }
        for account_id in [organization_owner_id, author_id, commenter_id] {
            assert!(
                authorization
                    .require_delete_comment(
                        principal(account_id),
                        Uuid::now_v7(),
                        commenter_id,
                        false,
                        facts(true, false, true),
                    )
                    .is_ok()
            );
            assert!(matches!(
                authorization.require_delete_comment(
                    principal(account_id),
                    Uuid::now_v7(),
                    commenter_id,
                    false,
                    facts(false, false, true),
                ),
                Err(ApplicationError::Forbidden)
            ));
        }
        assert!(matches!(
            authorization.require_delete_comment(
                principal(member_id),
                Uuid::now_v7(),
                commenter_id,
                false,
                facts(true, false, true),
            ),
            Err(ApplicationError::Forbidden)
        ));
    }

    #[test]
    fn api_clients_can_read_only_published_posts_in_their_own_organization() {
        let authorization = Authorization::new().unwrap();
        let client_id = Uuid::now_v7();
        let organization_id = Uuid::now_v7();
        let facts = |published, deleted| PostFacts {
            id: Uuid::now_v7(),
            organization_id,
            organization_owner_id: Uuid::now_v7(),
            author_id: Uuid::now_v7(),
            published,
            deleted,
            organization_member: false,
        };
        assert!(
            authorization
                .require_external_post(client_id, organization_id, facts(true, false),)
                .is_ok()
        );
        for post in [facts(false, false), facts(true, true)] {
            assert!(matches!(
                authorization.require_external_post(client_id, organization_id, post,),
                Err(ApplicationError::Forbidden)
            ));
        }
        assert!(matches!(
            authorization.require_external_post(client_id, Uuid::now_v7(), facts(true, false),),
            Err(ApplicationError::Forbidden)
        ));
    }
}
