use crate::{ApplicationError, ApplicationResult};
use cedar_policy::{
    Authorizer as CedarAuthorizer, Context, Decision, Entities, EntityUid, PolicySet, Request,
    Schema, ValidationMode, Validator,
};
use serde_json::json;
use uuid::Uuid;
mod content;
pub(crate) use content::PostFacts;

#[derive(Clone, Copy, Debug)]
pub struct Principal {
    pub account_id: Uuid,
    pub active: bool,
    pub verified: bool,
    pub administrator: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    ReadAccount,
    ManageAccount,
    ReadOrganization,
    ManageOrganization,
    AdministerAccount,
    CreatePost,
    ReadPost,
    EditPost,
    DeletePost,
    CreateComment,
    DeleteComment,
}
impl Action {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReadAccount => "read_account",
            Self::ManageAccount => "manage_account",
            Self::ReadOrganization => "read_organization",
            Self::ManageOrganization => "manage_organization",
            Self::AdministerAccount => "administer_account",
            Self::CreatePost => "create_post",
            Self::ReadPost => "read_post",
            Self::EditPost => "edit_post",
            Self::DeletePost => "delete_post",
            Self::CreateComment => "create_comment",
            Self::DeleteComment => "delete_comment",
        }
    }
}

pub struct Authorization {
    schema: Schema,
    policies: PolicySet,
}
impl Authorization {
    pub fn new() -> ApplicationResult<Self> {
        let schema = Schema::from_json_str(include_str!("schema.json"))
            .map_err(ApplicationError::internal)?;
        let policies: PolicySet = include_str!("policies.cedar")
            .parse()
            .map_err(ApplicationError::internal)?;
        if !Validator::new(schema.clone())
            .validate(&policies, ValidationMode::Strict)
            .validation_passed()
        {
            return Err(ApplicationError::internal(anyhow::anyhow!(
                "authorization policy does not match schema"
            )));
        }
        Ok(Self { schema, policies })
    }

    #[tracing::instrument(skip_all, fields(account_id = %principal.account_id, ?action, resource_id = %resource_id))]
    pub fn require(
        &self,
        principal: Principal,
        action: Action,
        resource_id: Uuid,
        owner_id: Uuid,
        organization_member: bool,
    ) -> ApplicationResult<()> {
        let action_uid: EntityUid = format!("Action::\"{}\"", action.as_str())
            .parse()
            .map_err(ApplicationError::internal)?;
        let principal_uid: EntityUid = format!("User::\"{}\"", principal.account_id)
            .parse()
            .map_err(ApplicationError::internal)?;
        let resource_uid: EntityUid = format!("Resource::\"{resource_id}\"")
            .parse()
            .map_err(ApplicationError::internal)?;
        let entities = Entities::from_json_value(json!([
            { "uid": {"type":"User","id":principal.account_id.to_string()}, "attrs": {
                "active":principal.active, "verified":principal.verified, "administrator":principal.administrator }, "parents":[] },
            { "uid":{"type":"Resource","id":resource_id.to_string()}, "attrs":{
                "owner":{"__entity":{"type":"User","id":owner_id.to_string()}} }, "parents":[] }
        ]), Some(&self.schema)).map_err(ApplicationError::internal)?;
        let context = Context::from_json_value(
            json!({"organization_member":organization_member}),
            Some((&self.schema, &action_uid)),
        )
        .map_err(ApplicationError::internal)?;
        let request = Request::new(
            principal_uid,
            action_uid,
            resource_uid,
            context,
            Some(&self.schema),
        )
        .map_err(ApplicationError::internal)?;
        let response = CedarAuthorizer::new().is_authorized(&request, &self.policies, &entities);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owners_members_administrators_and_suspension_obey_scope() {
        let authorization = Authorization::new().unwrap();
        let owner_id = Uuid::now_v7();
        let other_id = Uuid::now_v7();
        let principal = Principal {
            account_id: owner_id,
            active: true,
            verified: true,
            administrator: false,
        };
        assert!(
            authorization
                .require(principal, Action::ManageAccount, owner_id, owner_id, false)
                .is_ok()
        );
        assert!(
            authorization
                .require(principal, Action::ReadAccount, other_id, other_id, false)
                .is_err()
        );
        assert!(
            authorization
                .require(
                    principal,
                    Action::ReadOrganization,
                    other_id,
                    other_id,
                    true
                )
                .is_ok()
        );
        assert!(
            authorization
                .require(
                    principal,
                    Action::ReadOrganization,
                    other_id,
                    other_id,
                    false
                )
                .is_err()
        );
        let administrator = Principal {
            administrator: true,
            ..principal
        };
        assert!(
            authorization
                .require(
                    administrator,
                    Action::AdministerAccount,
                    other_id,
                    other_id,
                    false
                )
                .is_ok()
        );
        assert!(
            authorization
                .require(
                    administrator,
                    Action::ReadOrganization,
                    other_id,
                    other_id,
                    false
                )
                .is_err()
        );
        assert!(
            authorization
                .require(
                    Principal {
                        active: false,
                        ..administrator
                    },
                    Action::AdministerAccount,
                    other_id,
                    other_id,
                    false
                )
                .is_err()
        );
    }
}
