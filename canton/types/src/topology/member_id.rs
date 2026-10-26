use std::{fmt, str::FromStr};

use crate::topology::{
    MediatorId, MemberCode, Namespace, ParticipantId, SequencerId, UniqueIdentifier,
};

use super::errors::{UniqueIdentifierError, UnknownMemberCode};

/// Identifier of some member of the synchronizer: participant, mediator or sequencer
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemberId {
    Mediator(MediatorId),
    Participant(ParticipantId),
    Sequencer(SequencerId),
}

impl MemberId {
    pub fn member_code(&self) -> MemberCode {
        match self {
            MemberId::Mediator(_) => MediatorId::member_code(),
            MemberId::Participant(_) => ParticipantId::member_code(),
            MemberId::Sequencer(_) => SequencerId::member_code(),
        }
    }

    pub fn uid(&self) -> &UniqueIdentifier {
        match self {
            MemberId::Participant(participant_id) => participant_id.uid(),
            MemberId::Mediator(mediator_id) => mediator_id.uid(),
            MemberId::Sequencer(sequencer_id) => sequencer_id.uid(),
        }
    }

    pub fn identifier(&self) -> &str {
        self.uid().identifier()
    }

    pub fn namespace(&self) -> &Namespace {
        self.uid().namespace()
    }

    pub fn parse(s: &str) -> Result<Self, MemberIdError> {
        let (code, uid) = s
            .split_once(UniqueIdentifier::DELIMITER)
            .ok_or(MemberIdError {
                kind: ErrorKind::MissingMemberCode,
            })?;
        let code = MemberCode::parse(code).map_err(|error| MemberIdError {
            kind: ErrorKind::MemberCode(error),
        })?;
        let uid = UniqueIdentifier::parse(uid).map_err(|error| MemberIdError {
            kind: ErrorKind::UniqueIdentifier(error),
        })?;

        Ok(match code {
            MemberCode::Mediator => Self::Mediator(MediatorId::from_uid(uid)),
            MemberCode::Participant => Self::Participant(ParticipantId::from_uid(uid)),
            MemberCode::Sequencer => Self::Sequencer(SequencerId::from_uid(uid)),
        })
    }
}

impl AsRef<UniqueIdentifier> for MemberId {
    fn as_ref(&self) -> &UniqueIdentifier {
        self.uid()
    }
}

impl fmt::Display for MemberId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemberId::Mediator(mediator_id) => mediator_id.fmt(f),
            MemberId::Participant(participant_id) => participant_id.fmt(f),
            MemberId::Sequencer(sequencer_id) => sequencer_id.fmt(f),
        }
    }
}

impl FromStr for MemberId {
    type Err = MemberIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq<UniqueIdentifier> for MemberId {
    fn eq(&self, other: &UniqueIdentifier) -> bool {
        self.uid().eq(other)
    }
}

impl From<MediatorId> for MemberId {
    fn from(value: MediatorId) -> Self {
        Self::Mediator(value)
    }
}

impl From<ParticipantId> for MemberId {
    fn from(value: ParticipantId) -> Self {
        Self::Participant(value)
    }
}

impl From<SequencerId> for MemberId {
    fn from(value: SequencerId) -> Self {
        Self::Sequencer(value)
    }
}

impl From<MemberId> for UniqueIdentifier {
    fn from(value: MemberId) -> Self {
        match value {
            MemberId::Mediator(mediator_id) => mediator_id.into(),
            MemberId::Participant(participant_id) => participant_id.into(),
            MemberId::Sequencer(sequencer_id) => sequencer_id.into(),
        }
    }
}

impl From<MemberId> for String {
    fn from(value: MemberId) -> Self {
        match value {
            MemberId::Mediator(mediator_id) => mediator_id.into(),
            MemberId::Participant(participant_id) => participant_id.into(),
            MemberId::Sequencer(sequencer_id) => sequencer_id.into(),
        }
    }
}

impl TryFrom<String> for MemberId {
    type Error = MemberIdError;

    fn try_from(mut value: String) -> Result<Self, Self::Error> {
        let (code, _) = value
            .split_once(UniqueIdentifier::DELIMITER)
            .ok_or(MemberIdError {
                kind: ErrorKind::MissingMemberCode,
            })?;
        let code = MemberCode::parse(code).map_err(|error| MemberIdError {
            kind: ErrorKind::MemberCode(error),
        })?;
        let prefix_len = code.as_str().len() + UniqueIdentifier::DELIMITER.len();
        value.drain(..prefix_len);
        let uid = UniqueIdentifier::try_from(value).map_err(|error| MemberIdError {
            kind: ErrorKind::UniqueIdentifier(error),
        })?;

        Ok(match code {
            MemberCode::Mediator => Self::Mediator(MediatorId::from_uid(uid)),
            MemberCode::Participant => Self::Participant(ParticipantId::from_uid(uid)),
            MemberCode::Sequencer => Self::Sequencer(SequencerId::from_uid(uid)),
        })
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub struct MemberIdError {
    kind: ErrorKind,
}

#[derive(Clone, Debug, thiserror::Error)]
enum ErrorKind {
    #[error("member ID is missing a member code prefix")]
    MissingMemberCode,

    #[error(transparent)]
    MemberCode(#[from] UnknownMemberCode),

    #[error(transparent)]
    UniqueIdentifier(#[from] UniqueIdentifierError),
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::MemberId;

    #[rstest]
    #[case("MED::mediator::namespace")]
    #[case("PAR::participant::namespace")]
    #[case("SEQ::sequencer::namespace")]
    #[case("SEQ::_::_")]
    #[case("MED:::id::namespace:")]
    #[case("PAR::id:::namespace")]
    #[case("SEQ::foo:bar-baz_9::one:two")]
    #[case("PAR:: :: ")]
    #[case(&format!("PAR::{}::{}", "a".repeat(185), "b".repeat(68)))]
    #[should_panic(expected = "member ID is missing a member code prefix")]
    #[case("")]
    #[should_panic(expected = "member ID is missing a member code prefix")]
    #[case("PAR")]
    #[should_panic(expected = "member ID is missing a member code prefix")]
    #[case("PAR:id:ns")]
    #[should_panic(expected = "unknown member code")]
    #[case("id::ns")]
    #[should_panic(expected = "unknown member code")]
    #[case("par::id::ns")]
    #[should_panic(expected = "unknown member code")]
    #[case("XYZ::id::ns")]
    #[should_panic(expected = "unknown member code")]
    #[case("PART::id::ns")]
    #[should_panic(expected = "unknown member code")]
    #[case("::id::ns")]
    #[should_panic(expected = "unknown member code")]
    #[case("ਊ::id::ns")]
    #[should_panic(expected = "empty first part")]
    #[case("MED::")]
    #[should_panic(expected = "empty first part")]
    #[case("PAR::::ns")]
    #[should_panic(expected = "missing a namespace")]
    #[case("SEQ::id")]
    #[should_panic(expected = "namespace is empty")]
    #[case("PAR::id::")]
    #[should_panic(expected = "unexpected character '%' in unique identifier")]
    #[case("MED::bad%::ns")]
    #[should_panic(expected = "unexpected character 'ਊ' in unique identifier")]
    #[case("SEQ::ਊ::ns")]
    #[should_panic(expected = "unexpected character '%' in namespace")]
    #[case("PAR::id::bad%")]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("MED::id::ਊ")]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case("SEQ::id::ns::extra")]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case("PAR::id::::ns")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case(&format!("PAR::{}::ns", "a".repeat(186)))]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case(&format!("SEQ::id::{}", "b".repeat(69)))]
    fn test_parse(#[case] input: &str) {
        if let Err(error) = MemberId::parse(input) {
            panic!("{error}")
        }
    }
}
