use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Deserialize,
    Serialize,
    ToSchema,
    Hash,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "role")]
pub enum Role {
    #[sea_orm(string_value = "mod")]
    Mod,
    #[sea_orm(string_value = "admin")]
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Admin => "admin",
        }
    }

    /// Higher value = higher priority (Admin > Mod)
    pub fn display_priority(self) -> u8 {
        match self {
            Self::Mod => 1,
            Self::Admin => 2,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mod" => Ok(Self::Mod),
            "admin" => Ok(Self::Admin),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}
