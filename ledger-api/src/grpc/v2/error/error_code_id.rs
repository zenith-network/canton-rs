//! Some of well-known Canton error code IDs.
//!
//! This is not a complete list.

// TODO: I'm not sure it's a good approach to just store common codes here as string constants.
//       Maybe we should have a enum for that instead. However the problem is that this enum will be
//       very big. Total number of all different codes is now > 300.

// Request validation / malformed input
pub const MALFORMED_REQUEST: &str = "MALFORMED_REQUEST";
pub const MISSING_FIELD: &str = "MISSING_FIELD";
pub const INVALID_ARGUMENT: &str = "INVALID_ARGUMENT";
pub const INVALID_FIELD: &str = "INVALID_FIELD";
pub const INVALID_TOKEN: &str = "INVALID_TOKEN";
pub const UNKNOWN_RESOURCE: &str = "UNKNOWN_RESOURCE";
pub const OFFSET_AFTER_LEDGER_END: &str = "OFFSET_AFTER_LEDGER_END";
pub const PACKAGE_NOT_FOUND: &str = "PACKAGE_NOT_FOUND";
pub const UPDATE_NOT_FOUND: &str = "UPDATE_NOT_FOUND";
pub const PACKAGE_SELECTION_FAILED: &str = "PACKAGE_SELECTION_FAILED";

// Auth / permissions
pub const UNAUTHENTICATED: &str = "UNAUTHENTICATED";
pub const ACCESS_TOKEN_EXPIRED: &str = "ACCESS_TOKEN_EXPIRED";
pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";

// Submission lifecycle / duplicate handling
pub const SUBMISSION_ALREADY_IN_FLIGHT: &str = "SUBMISSION_ALREADY_IN_FLIGHT";
pub const DUPLICATE_COMMAND: &str = "DUPLICATE_COMMAND";
pub const REQUEST_TIME_OUT: &str = "REQUEST_TIME_OUT";
pub const REQUEST_DEADLINE_EXCEEDED: &str = "REQUEST_DEADLINE_EXCEEDED";

// Backpressure / overload / transport contention
pub const PARTICIPANT_BACKPRESSURE: &str = "PARTICIPANT_BACKPRESSURE";
pub const SEQUENCER_BACKPRESSURE: &str = "SEQUENCER_BACKPRESSURE";
pub const SEQUENCER_REQUEST_FAILED: &str = "SEQUENCER_REQUEST_FAILED";
pub const NOT_SEQUENCED_TIMEOUT: &str = "NOT_SEQUENCED_TIMEOUT";
pub const LOCAL_VERDICT_TIMEOUT: &str = "LOCAL_VERDICT_TIMEOUT";
pub const MEDIATOR_SAYS_TX_TIMED_OUT: &str = "MEDIATOR_SAYS_TX_TIMED_OUT";

// Contention / race outcomes on contracts and keys
pub const LOCAL_VERDICT_LOCKED_CONTRACTS: &str = "LOCAL_VERDICT_LOCKED_CONTRACTS";
pub const LOCAL_VERDICT_INACTIVE_CONTRACTS: &str = "LOCAL_VERDICT_INACTIVE_CONTRACTS";
pub const CONTRACT_NOT_FOUND: &str = "CONTRACT_NOT_FOUND";
pub const DUPLICATE_CONTRACT_KEY: &str = "DUPLICATE_CONTRACT_KEY";
pub const INCONSISTENT_CONTRACT_KEY: &str = "INCONSISTENT_CONTRACT_KEY";

// Topology / package vetting
pub const PACKAGE_NOT_VETTED_BY_RECIPIENTS: &str = "PACKAGE_NOT_VETTED_BY_RECIPIENTS";

// Connectivity / synchronizer availability
pub const NOT_CONNECTED_TO_ANY_SYNCHRONIZER: &str = "NOT_CONNECTED_TO_ANY_SYNCHRONIZER";
pub const NOT_CONNECTED_TO_SYNCHRONIZER: &str = "NOT_CONNECTED_TO_SYNCHRONIZER";
pub const SYNCHRONIZER_IS_NOT_AVAILABLE: &str = "SYNCHRONIZER_IS_NOT_AVAILABLE";
