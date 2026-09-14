use crate::topology::{MediatorId, ParticipantId, SequencerId};

/// Identifier of some member of the syncrhonizer: participant, mediator or sequencer
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemberId {
    Participant(ParticipantId),
    Mediator(MediatorId),
    Sequencer(SequencerId),
}
