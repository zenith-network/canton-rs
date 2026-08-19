use std::{fmt, time::Duration};

// Re-export for convenience (this type is exposed in public API of the module)
pub use tonic::Code;

/// Error category ID
///
/// A small integer identifying the corresponding error category.
///
/// This is a broad categorization of error codes that you can base your error handling strategies
/// on. Maps to exactly one `gRPC status code`_. We recommend dealing with errors based on their
/// error category. However, if the error category alone is too generic you can act on a particular
/// error code.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CategoryId {
    /// Service is temporarily unavailable
    ///
    /// One of the services required to process the request was not available. The request might or
    /// might not have been processed, as the server aborted the request while it was being
    /// processed. Note that for requests that change the state of the system, this error may be
    /// returned even if the request has completed successfully.
    ///
    /// # Resolution
    ///
    /// Expectation: transient failure that should be handled by retrying the request with
    /// appropriate backoff.
    ///
    /// # Retry strategy
    ///
    /// Retry quickly in load balancer.
    TransientServerFailure,

    /// Failure due to contention on some resources
    ///
    /// The request could not be processed due to shared processing resources (e.g. locks or rate
    /// limits that replenish quickly) being occupied. If the resource is known (i.e. locked
    /// contract), it will be included as a resource info. (Not known resource contentions are e.g.
    /// overloaded networks where we just observe timeouts, but can’t pin-point the cause).
    ///
    /// # Resolution
    ///
    /// Expectation: this is processing-flow level contention that should be handled by retrying the
    /// request with appropriate backoff.
    ///
    /// # Retry strategy
    ///
    /// Retry quickly (indefinitely or limited), but do not retry in load balancer.
    ContentionOnSharedResources,

    /// Request completion not observed within a pre-defined window
    ///
    /// The request might not have been processed, as its deadline expired before its completion was
    /// signalled. Note that for requests that change the state of the system, this error may be
    /// returned even if the request has completed successfully. Note that known and well-defined
    /// timeouts are signalled as [`Self::ContentionOnSharedResources`], while this category
    /// indicates that the state of the request is unknown.
    ///
    /// # Resolution
    ///
    /// Expectation: the deadline might have been exceeded due to transient resource congestion or
    /// due to a timeout in the request processing pipeline being too low. The transient errors
    /// might be solved by the application retrying. The non-transient errors will require operator
    /// intervention to change the timeouts.
    ///
    /// # Retry strategy
    ///
    /// Retry for a limited number of times with deduplication.
    DeadlineExceededRequestStateUnknown,

    /// Some internal error
    ///
    /// Request processing failed due to a violation of system internal invariants. This error is
    /// exposed on the API with gRPC code [`INTERNAL`][Code::Internal] without any details for
    /// security reasons.
    ///
    /// # Resolution
    ///
    /// Expectation: this is due to a bug in the implementation or data corruption in the systems
    /// databases. Resolution will require operator intervention, and potentially vendor support.
    ///
    /// # Retry strategy
    ///
    /// Retry after operator intervention.
    SystemInternalAssumptionViolated,

    /// A potential attack or a faulty peer component has been detected. This error is exposed on
    /// the API with gRPC code [`INVALID_ARGUMENT`][Code::InvalidArgument] without any details for
    /// security reasons.
    ///
    /// # Resolution
    ///
    /// Expectation: this can be a severe issue that requires operator attention or intervention,
    /// and potentially vendor support. It means that the system has detected invalid information
    /// that can be attributed to either faulty or malicious manipulation of data coming from a peer
    /// source.
    ///
    /// # Retry strategy
    ///
    /// Errors in this category are non-retryable.
    SecurityAlert,

    /// Client is not authenticated properly
    ///
    /// The request does not have valid authentication credentials for the operation. This error is
    /// exposed on the API with gRPC code [`UNAUTHENTICATED`][Code::Unauthenticated] without any
    /// details for security reasons.
    ///
    /// # Resolution
    ///
    /// Expectation: this is an application bug, application misconfiguration or ledger-level
    /// misconfiguration. Resolution requires application and/or ledger operator intervention.
    ///
    /// # Retry strategy
    ///
    /// Retry after application operator intervention.
    AuthInterceptorInvalidAuthenticationCredentials,

    /// Client does not have appropriate permissions
    ///
    /// The caller does not have permission to execute the specified operation. This error is
    /// exposed on the API with gRPC code [`PERMISSION_DENIED`][Code::PermissionDenied] without any
    /// details for security reasons.
    ///
    /// # Resolution
    ///
    /// Expectation: this is an application bug or application misconfiguration. Resolution requires
    /// application operator intervention.
    ///
    /// # Retry strategy
    ///
    /// Retry after application operator intervention.
    InsufficientPermission,

    /// A request which is never going to be valid
    ///
    /// The request is invalid independent of the state of the system.
    ///
    /// # Resolution
    ///
    /// Expectation: this is an application bug or ledger-level misconfiguration (e.g. request size
    /// limits). Resolution requires application and/or ledger operator intervention.
    ///
    /// # Retry strategy
    ///
    /// Retry after application operator intervention.
    InvalidIndependentOfSystemState,

    /// A failure due to the current system state
    ///
    /// The mutable state of the system does not satisfy the preconditions required to execute the
    /// request. We consider the whole Daml ledger including ledger config, parties, packages, users
    /// and command deduplication to be mutable system state. Thus all Daml interpretation errors
    /// are reported as this error or one of its specializations.
    ///
    /// # Resolution
    ///
    /// [`ALREADY_EXISTS`][Code::AlreadyExists] and [`NOT_FOUND`][Code::NotFound] are special cases
    /// for the existence and non-existence of well-defined entities within the system state; e.g.,
    /// a .dalf package, contracts ids, contract keys, or a transaction at an offset.
    /// [`OUT_OF_RANGE`][Code::OutOfRange] is a special case for reading past a range. Violations of
    /// the Daml ledger model always result in these kinds of errors.
    ///
    /// Expectation: this is due to application-level bugs, misconfiguration or contention on
    /// application-visible resources; and might be resolved by retrying later, or after changing
    /// the state of the system. Handling these errors requires an application-specific strategy
    /// and/or operator intervention.
    ///
    /// # Retry strategy
    ///
    /// Retry after application operator intervention.
    InvalidGivenCurrentSystemStateOther,

    /// A failure due to a resource already existing in the current system state
    ///
    /// Special type of InvalidGivenCurrentSystemState referring to a well-defined resource.
    ///
    /// # Resolution
    ///
    /// Same as [`Self::InvalidGivenCurrentSystemStateOther`].
    ///
    /// # Retry strategy
    ///
    /// Inspect resource failure and retry after resource failure has been resolved (depends on type
    /// of resource and application).
    InvalidGivenCurrentSystemStateResourceExists,

    /// A failure due to a resource not existing in the current system state
    ///
    /// Special type of InvalidGivenCurrentSystemState referring to a well-defined resource.
    ///
    /// # Resolution
    ///
    /// Same as [`Self::InvalidGivenCurrentSystemStateOther`].
    ///
    /// # Retry strategy
    ///
    /// Inspect resource failure and retry after resource failure has been resolved (depends on type
    /// of resource and application).
    InvalidGivenCurrentSystemStateResourceMissing,

    /// A failure due to requesting a resource using a parameter value that falls beyond the current
    /// upper bound (or 'end') defined by the system's state.
    ///
    /// The request failed because it resulted in an operation beyond the current upper bound (or
    /// 'end') defined by the system's state. For example, supplying a ledger offset which is larger
    /// than the current ledger end, or a record time that is in the future.
    ///
    /// # Resolution
    ///
    /// Resolution can occur naturally as the system progresses. The requested operation may become
    /// valid eventually once the system's state has advanced further. For example, when new ledger
    /// entries are added. If however the situation does not resolve as expected, operator
    /// intervention may be required.
    ///
    /// # Retry strategy
    ///
    /// Wait and retry. For example, retry a limited number of times with potentially increasing
    /// backoff.
    ///
    /// Hint: Inspect the retryable value of the error code to decide on the particular
    /// retry duration.
    InvalidGivenCurrentSystemStateSeekAfterEnd,

    /// This error category is used to signal that an unimplemented code-path has been triggered by
    /// a client or participant operator request. This error is exposed on the API with grpc-status
    /// [`UNIMPLEMENTED`][Code::Unimplemented] without any details for security reasons.
    ///
    /// # Resolution
    ///
    /// This error is caused by a ledger-level misconfiguration or by an implementation bug.
    /// Resolution requires node operator intervention.
    ///
    /// # Retry strategy
    ///
    /// Errors in this category are non-retryable.
    InternalUnsupportedOperation,

    /// Unknown category
    ///
    /// Fallback variant. No certain information about resolution or retry strategy.
    ///
    /// This variant should be considered as unexpected behavior of Canton API.
    Unknown(UnknownCategoryId),
}

impl CategoryId {
    /// Default retryability information for this error category
    pub const fn retry(&self) -> Option<Duration> {
        match self {
            CategoryId::TransientServerFailure => Some(Duration::from_secs(1)),
            CategoryId::ContentionOnSharedResources => Some(Duration::from_secs(1)),
            CategoryId::DeadlineExceededRequestStateUnknown => Some(Duration::from_secs(1)),
            CategoryId::SystemInternalAssumptionViolated => None,
            CategoryId::SecurityAlert => None,
            CategoryId::AuthInterceptorInvalidAuthenticationCredentials => None,
            CategoryId::InsufficientPermission => None,
            CategoryId::InvalidIndependentOfSystemState => None,
            CategoryId::InvalidGivenCurrentSystemStateOther => None,
            CategoryId::InvalidGivenCurrentSystemStateResourceExists => None,
            CategoryId::InvalidGivenCurrentSystemStateResourceMissing => None,
            CategoryId::InvalidGivenCurrentSystemStateSeekAfterEnd => Some(Duration::from_secs(1)),
            CategoryId::InternalUnsupportedOperation => None,
            CategoryId::Unknown(_) => None,
        }
    }

    /// The gRPC code use to signal this error
    pub const fn code(&self) -> Code {
        match self {
            CategoryId::TransientServerFailure => Code::Unavailable,
            CategoryId::ContentionOnSharedResources => Code::Aborted,
            CategoryId::DeadlineExceededRequestStateUnknown => Code::DeadlineExceeded,
            CategoryId::SystemInternalAssumptionViolated => Code::Internal,
            CategoryId::SecurityAlert => Code::InvalidArgument,
            CategoryId::AuthInterceptorInvalidAuthenticationCredentials => Code::Unauthenticated,
            CategoryId::InsufficientPermission => Code::PermissionDenied,
            CategoryId::InvalidIndependentOfSystemState => Code::InvalidArgument,
            CategoryId::InvalidGivenCurrentSystemStateOther => Code::FailedPrecondition,
            CategoryId::InvalidGivenCurrentSystemStateResourceExists => Code::AlreadyExists,
            CategoryId::InvalidGivenCurrentSystemStateResourceMissing => Code::NotFound,
            CategoryId::InvalidGivenCurrentSystemStateSeekAfterEnd => Code::OutOfRange,
            CategoryId::InternalUnsupportedOperation => Code::Unimplemented,
            CategoryId::Unknown(_) => Code::Unknown,
        }
    }

    /// Convert from `i32`
    pub const fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::TransientServerFailure,
            2 => Self::ContentionOnSharedResources,
            3 => Self::DeadlineExceededRequestStateUnknown,
            4 => Self::SystemInternalAssumptionViolated,
            5 => Self::SecurityAlert,
            6 => Self::AuthInterceptorInvalidAuthenticationCredentials,
            7 => Self::InsufficientPermission,
            8 => Self::InvalidIndependentOfSystemState,
            9 => Self::InvalidGivenCurrentSystemStateOther,
            10 => Self::InvalidGivenCurrentSystemStateResourceExists,
            11 => Self::InvalidGivenCurrentSystemStateResourceMissing,
            12 => Self::InvalidGivenCurrentSystemStateSeekAfterEnd,
            // 13 is skipped for BackgroundProcessDegradationWarning, which is not used in API
            14 => Self::InternalUnsupportedOperation,
            unknown => Self::Unknown(UnknownCategoryId(unknown)),
        }
    }

    /// Return ID as `i32`
    pub const fn as_i32(&self) -> i32 {
        match self {
            CategoryId::TransientServerFailure => 1,
            CategoryId::ContentionOnSharedResources => 2,
            CategoryId::DeadlineExceededRequestStateUnknown => 3,
            CategoryId::SystemInternalAssumptionViolated => 4,
            CategoryId::SecurityAlert => 5,
            CategoryId::AuthInterceptorInvalidAuthenticationCredentials => 6,
            CategoryId::InsufficientPermission => 7,
            CategoryId::InvalidIndependentOfSystemState => 8,
            CategoryId::InvalidGivenCurrentSystemStateOther => 9,
            CategoryId::InvalidGivenCurrentSystemStateResourceExists => 10,
            CategoryId::InvalidGivenCurrentSystemStateResourceMissing => 11,
            CategoryId::InvalidGivenCurrentSystemStateSeekAfterEnd => 12,
            CategoryId::InternalUnsupportedOperation => 14,
            CategoryId::Unknown(unknown) => unknown.0,
        }
    }

    /// Return a human-readable description of the category
    pub const fn description(&self) -> &'static str {
        match self {
            CategoryId::TransientServerFailure => "transient server failure",
            CategoryId::ContentionOnSharedResources => "contention on shared resources",
            CategoryId::DeadlineExceededRequestStateUnknown => {
                "deadline exceeded (request state unknown)"
            }
            CategoryId::SystemInternalAssumptionViolated => "system internal assumption violated",
            CategoryId::SecurityAlert => "security alert",
            CategoryId::AuthInterceptorInvalidAuthenticationCredentials => {
                "invalid authentication credentials"
            }
            CategoryId::InsufficientPermission => "insufficient permission",
            CategoryId::InvalidIndependentOfSystemState => "invalid independent of system state",
            CategoryId::InvalidGivenCurrentSystemStateOther => {
                "invalid given current system state (other)"
            }
            CategoryId::InvalidGivenCurrentSystemStateResourceExists => {
                "invalid given current system state (resource exists)"
            }
            CategoryId::InvalidGivenCurrentSystemStateResourceMissing => {
                "invalid given current system state (resource missing)"
            }
            CategoryId::InvalidGivenCurrentSystemStateSeekAfterEnd => {
                "invalid given current system state (seek after end)"
            }
            CategoryId::InternalUnsupportedOperation => "internal unsupported operation",
            CategoryId::Unknown(_) => "unknown category",
        }
    }
}

impl From<i32> for CategoryId {
    fn from(value: i32) -> Self {
        Self::from_i32(value)
    }
}

impl From<CategoryId> for i32 {
    fn from(value: CategoryId) -> Self {
        value.as_i32()
    }
}

impl fmt::Display for CategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.description())
        } else {
            self.as_i32().fmt(f)
        }
    }
}

/// Unknown category ID.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownCategoryId(i32);

impl fmt::Display for UnknownCategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for UnknownCategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<UnknownCategoryId> for i32 {
    fn from(value: UnknownCategoryId) -> Self {
        value.0
    }
}
