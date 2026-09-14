use std::fmt;

/// Code of the member of the synchronizer: participant, mediator or sequencer
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemberCode {
    Participant,
    Mediator,
    Sequencer,
}

impl MemberCode {
    /// Three-letter representation of the code (`"PAR"`, `"MED"`, `"SEQ"`)
    pub const fn as_str(&self) -> &'static str {
        match self {
            MemberCode::Participant => "PAR",
            MemberCode::Mediator => "MED",
            MemberCode::Sequencer => "SEQ",
        }
    }

    pub fn parse(s: &str) -> Result<Self, UnknownMemberCode> {
        match s {
            "PAR" => Ok(Self::Participant),
            "MED" => Ok(Self::Mediator),
            "SEQ" => Ok(Self::Sequencer),
            _ => Err(UnknownMemberCode { code: s.to_owned() }),
        }
    }
}

impl fmt::Display for MemberCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("unknown member code: {code:?}")]
pub struct UnknownMemberCode {
    code: String,
}
