use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ApplicationEnvironment {
    Development,
    Production,
    Test,
}

impl FromStr for ApplicationEnvironment {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            "test" => Ok(Self::Test),
            _ => Err(anyhow::anyhow!("invalid APP_ENV")),
        }
    }
}
