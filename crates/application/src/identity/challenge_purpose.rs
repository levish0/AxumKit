use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum ChallengePurpose {
    #[sea_orm(string_value = "verify_email")]
    VerifyEmail,
    #[sea_orm(string_value = "reset_password")]
    ResetPassword,
    #[sea_orm(string_value = "totp_setup")]
    TotpSetup,
    #[sea_orm(string_value = "mfa")]
    Mfa,
}
