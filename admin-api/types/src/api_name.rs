use std::{fmt, str::FromStr};

/// Name of the API. Helps detect invalid port configurations.
///
/// Returned from `ApiInfoService` service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ApiName {
    /// Admin API (participant, sequencer or mediator)
    AdminApi,

    /// Participant Ledger API
    LedgerApi,

    /// Sequencer public API
    SequencerPublicApi,
}

impl ApiName {
    /// Parse API name from string
    pub fn parse(s: &str) -> Result<Self, UnknownApiName> {
        match s {
            "admin-api" => Ok(Self::AdminApi),
            "ledger-api" => Ok(Self::LedgerApi),
            "sequencer-public-api" => Ok(Self::SequencerPublicApi),
            _ => Err(UnknownApiName { name: s.to_owned() }),
        }
    }

    /// String representation of API name
    pub const fn as_str(&self) -> &'static str {
        match self {
            ApiName::AdminApi => "admin-api",
            ApiName::LedgerApi => "ledger-api",
            ApiName::SequencerPublicApi => "sequencer-public-api",
        }
    }
}

impl fmt::Display for ApiName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ApiName {
    type Err = UnknownApiName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for ApiName {
    type Error = UnknownApiName;

    /// Same as [`Self::parse`], but avoids cloning of the string on error.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "admin-api" => Ok(Self::AdminApi),
            "ledger-api" => Ok(Self::LedgerApi),
            "sequencer-public-api" => Ok(Self::SequencerPublicApi),
            _ => Err(UnknownApiName { name: value }),
        }
    }
}

impl From<ApiName> for String {
    fn from(value: ApiName) -> Self {
        value.to_string()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown API name: '{name}'")]
pub struct UnknownApiName {
    name: String,
}
