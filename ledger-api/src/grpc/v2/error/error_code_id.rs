use std::fmt;

/// Some of well-known Canton error code IDs.
///
/// This is not a complete list.
///
/// Originally this is unique non-empty string containing at most 63 characters: upper-case letters,
/// underscores or digits.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ErrorCodeId {
    // Request validation / malformed input
    MalformedRequest,
    MissingField,
    InvalidArgument,
    InvalidField,
    InvalidToken,
    UnknownResource,
    OffsetAfterLedgerEnd,
    PackageNotFound,
    UpdateNotFound,
    PackageSelectionFailed,

    // Auth / permissions
    Unauthenticated,
    AccessTokenExpired,
    PermissionDenied,

    // Submission lifecycle / duplicate handling
    SubmissionAlreadyInFlight,
    DuplicateCommand,
    RequestTimeOut,
    RequestDeadlineExceeded,

    // Backpressure / overload / transport contention
    ParticipantBackpressure,
    SequencerBackpressure,
    SequencerRequestFailed,
    NotSequencedTimeout,
    LocalVerdictTimeout,
    MediatorSaysTxTimedOut,

    // Contention / race outcomes on contracts and keys
    LocalVerdictLockedContracts,
    LocalVerdictInactiveContracts,
    ContractNotFound,
    DuplicateContractKey,
    InconsistentContractKey,

    // Topology / package vetting
    PackageNotVettedByRecipients,

    // Connectivity / synchronizer availability
    NotConnectedToAnySynchronizer,
    NotConnectedToSynchronizer,
    SynchronizerIsNotAvailable,

    /// Fallback variant when converting from a raw string
    Unknown(String),
}

impl ErrorCodeId {
    pub fn as_str(&self) -> &str {
        match self {
            Self::MalformedRequest => "MALFORMED_REQUEST",
            Self::MissingField => "MISSING_FIELD",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::InvalidField => "INVALID_FIELD",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::UnknownResource => "UNKNOWN_RESOURCE",
            Self::OffsetAfterLedgerEnd => "OFFSET_AFTER_LEDGER_END",
            Self::PackageNotFound => "PACKAGE_NOT_FOUND",
            Self::UpdateNotFound => "UPDATE_NOT_FOUND",
            Self::PackageSelectionFailed => "PACKAGE_SELECTION_FAILED",
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::AccessTokenExpired => "ACCESS_TOKEN_EXPIRED",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::SubmissionAlreadyInFlight => "SUBMISSION_ALREADY_IN_FLIGHT",
            Self::DuplicateCommand => "DUPLICATE_COMMAND",
            Self::RequestTimeOut => "REQUEST_TIME_OUT",
            Self::RequestDeadlineExceeded => "REQUEST_DEADLINE_EXCEEDED",
            Self::ParticipantBackpressure => "PARTICIPANT_BACKPRESSURE",
            Self::SequencerBackpressure => "SEQUENCER_BACKPRESSURE",
            Self::SequencerRequestFailed => "SEQUENCER_REQUEST_FAILED",
            Self::NotSequencedTimeout => "NOT_SEQUENCED_TIMEOUT",
            Self::LocalVerdictTimeout => "LOCAL_VERDICT_TIMEOUT",
            Self::MediatorSaysTxTimedOut => "MEDIATOR_SAYS_TX_TIMED_OUT",
            Self::LocalVerdictLockedContracts => "LOCAL_VERDICT_LOCKED_CONTRACTS",
            Self::LocalVerdictInactiveContracts => "LOCAL_VERDICT_INACTIVE_CONTRACTS",
            Self::ContractNotFound => "CONTRACT_NOT_FOUND",
            Self::DuplicateContractKey => "DUPLICATE_CONTRACT_KEY",
            Self::InconsistentContractKey => "INCONSISTENT_CONTRACT_KEY",
            Self::PackageNotVettedByRecipients => "PACKAGE_NOT_VETTED_BY_RECIPIENTS",
            Self::NotConnectedToAnySynchronizer => "NOT_CONNECTED_TO_ANY_SYNCHRONIZER",
            Self::NotConnectedToSynchronizer => "NOT_CONNECTED_TO_SYNCHRONIZER",
            Self::SynchronizerIsNotAvailable => "SYNCHRONIZER_IS_NOT_AVAILABLE",
            Self::Unknown(s) => s.as_str(),
        }
    }

    pub fn from_string(s: String) -> Self {
        match s.as_str() {
            "MALFORMED_REQUEST" => Self::MalformedRequest,
            "MISSING_FIELD" => Self::MissingField,
            "INVALID_ARGUMENT" => Self::InvalidArgument,
            "INVALID_FIELD" => Self::InvalidField,
            "INVALID_TOKEN" => Self::InvalidToken,
            "UNKNOWN_RESOURCE" => Self::UnknownResource,
            "OFFSET_AFTER_LEDGER_END" => Self::OffsetAfterLedgerEnd,
            "PACKAGE_NOT_FOUND" => Self::PackageNotFound,
            "UPDATE_NOT_FOUND" => Self::UpdateNotFound,
            "PACKAGE_SELECTION_FAILED" => Self::PackageSelectionFailed,
            "UNAUTHENTICATED" => Self::Unauthenticated,
            "ACCESS_TOKEN_EXPIRED" => Self::AccessTokenExpired,
            "PERMISSION_DENIED" => Self::PermissionDenied,
            "SUBMISSION_ALREADY_IN_FLIGHT" => Self::SubmissionAlreadyInFlight,
            "DUPLICATE_COMMAND" => Self::DuplicateCommand,
            "REQUEST_TIME_OUT" => Self::RequestTimeOut,
            "REQUEST_DEADLINE_EXCEEDED" => Self::RequestDeadlineExceeded,
            "PARTICIPANT_BACKPRESSURE" => Self::ParticipantBackpressure,
            "SEQUENCER_BACKPRESSURE" => Self::SequencerBackpressure,
            "SEQUENCER_REQUEST_FAILED" => Self::SequencerRequestFailed,
            "NOT_SEQUENCED_TIMEOUT" => Self::NotSequencedTimeout,
            "LOCAL_VERDICT_TIMEOUT" => Self::LocalVerdictTimeout,
            "MEDIATOR_SAYS_TX_TIMED_OUT" => Self::MediatorSaysTxTimedOut,
            "LOCAL_VERDICT_LOCKED_CONTRACTS" => Self::LocalVerdictLockedContracts,
            "LOCAL_VERDICT_INACTIVE_CONTRACTS" => Self::LocalVerdictInactiveContracts,
            "CONTRACT_NOT_FOUND" => Self::ContractNotFound,
            "DUPLICATE_CONTRACT_KEY" => Self::DuplicateContractKey,
            "INCONSISTENT_CONTRACT_KEY" => Self::InconsistentContractKey,
            "PACKAGE_NOT_VETTED_BY_RECIPIENTS" => Self::PackageNotVettedByRecipients,
            "NOT_CONNECTED_TO_ANY_SYNCHRONIZER" => Self::NotConnectedToAnySynchronizer,
            "NOT_CONNECTED_TO_SYNCHRONIZER" => Self::NotConnectedToSynchronizer,
            "SYNCHRONIZER_IS_NOT_AVAILABLE" => Self::SynchronizerIsNotAvailable,
            _ => Self::Unknown(s),
        }
    }
}

impl fmt::Display for ErrorCodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
