pub mod grpc;

pub use ledger_api_types as types;

// Re-export
pub use canton_version;
pub use tonic;
pub use tower;

// Re-export for convenience
pub use canton_version::CANTON_VERSION;

use canton_version::semver::{Comparator, Op, Prerelease, Version, VersionReq};

/// Returns a list of version requirements, which defines versions of the Ledger API, compatible
/// with this client.
///
/// For Ledger API we follow the semver compatibility rules. E.g. if `canton-version` reports
/// `3.5.12`, then any version `^3.5.12` (`>=3.5.12 AND <4.0.0`) is considered compatible.
///
/// If any pre-release version is considered compatible, it needs to be added here manually. Other
/// pre-release versions are rejected.
pub fn supported_versions() -> Vec<VersionReq> {
    vec![VersionReq {
        comparators: vec![Comparator {
            op: Op::Caret,
            major: CANTON_VERSION.major,
            minor: Some(CANTON_VERSION.minor),
            patch: Some(CANTON_VERSION.patch),
            pre: Prerelease::EMPTY,
        }],
    }]
}

/// Check given version against [`supported_versions()`] and return `true`, if it's supported.
pub fn is_supported_version(version: &Version) -> bool {
    supported_versions().iter().any(|req| req.matches(version))
}
