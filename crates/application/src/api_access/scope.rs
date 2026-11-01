use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(64))")]
#[serde(rename_all = "snake_case")]
pub enum ApiScope {
    #[sea_orm(string_value = "organizations:read")]
    #[serde(rename = "organizations:read")]
    OrganizationsRead,
    #[sea_orm(string_value = "posts:read")]
    #[serde(rename = "posts:read")]
    PostsRead,
}

impl ApiScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrganizationsRead => "organizations:read",
            Self::PostsRead => "posts:read",
        }
    }
}
