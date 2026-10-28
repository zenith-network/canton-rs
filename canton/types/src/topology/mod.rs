//! Types related to Canton topology
//!
//! Note that some identifiers here are duplicating the root types. That's because the root types
//! are the types accepted by the Ledger API. They are missing the internal structure of the
//! identifiers. This module provides structured identifiers, which are mostly used by Admin API.

// TODO: remove last note after refactoring

mod fingerprint;
mod mediator_id;
mod member_code;
mod member_id;
mod namespace;
mod participant_id;
mod party_id;
mod physical_synchronizer_id;
mod protocol_version;
mod sequencer_id;
mod synchronizer_id;
mod unique_identifier;

pub use fingerprint::Fingerprint;
pub use mediator_id::MediatorId;
pub use member_code::MemberCode;
pub use member_id::MemberId;
pub use namespace::Namespace;
pub use participant_id::ParticipantId;
pub use party_id::PartyId;
pub use physical_synchronizer_id::PhysicalSynchronizerId;
pub use protocol_version::ProtocolVersion;
pub use sequencer_id::SequencerId;
pub use synchronizer_id::SynchronizerId;
pub use unique_identifier::UniqueIdentifier;

/// Error types
pub mod errors {
    pub use super::fingerprint::FingerprintError;
    pub use super::mediator_id::MediatorIdError;
    pub use super::member_code::UnknownMemberCode;
    pub use super::member_id::MemberIdError;
    pub use super::namespace::NamespaceError;
    pub use super::participant_id::ParticipantIdError;
    pub use super::party_id::PartyIdError;
    pub use super::physical_synchronizer_id::PhysicalSynchronizerIdError;
    pub use super::protocol_version::ProtocolVersionError;
    pub use super::sequencer_id::SequencerIdError;
    pub use super::synchronizer_id::SynchronizerIdError;
    pub use super::unique_identifier::UniqueIdentifierError;
}
