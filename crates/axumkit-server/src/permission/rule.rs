use crate::permission::UserContext;
use axumkit_errors::errors::Errors;

pub trait Rule {
    fn check(&self, ctx: &UserContext) -> Result<(), Errors>;

    fn is_allowed(&self, ctx: &UserContext) -> bool {
        self.check(ctx).is_ok()
    }
}
