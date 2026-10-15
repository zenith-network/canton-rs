//! Types related to Canton topology
//!
//! Note that some identifiers here are duplicating the root types. That's because the root types
//! are the types accepted by the Ledger API. They are missing the internal structure of the
//! identifiers. This module provides structured identifiers, which are mostly used by Admin API.

mod fingerprint;
mod identifier;
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

pub use fingerprint::Fingerprint;
pub use identifier::Identifier;
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

/// Error types
pub mod errors {
    pub use super::identifier::IdentifierError;
    pub use super::mediator_id::MediatorIdError;
    pub use super::member_code::UnknownMemberCode;
    pub use super::namespace::NamespaceError;
    pub use super::participant_id::ParticipantIdError;
    pub use super::physical_synchronizer_id::PhysicalSynchronizerIdError;
    pub use super::sequencer_id::SequencerIdError;
}
