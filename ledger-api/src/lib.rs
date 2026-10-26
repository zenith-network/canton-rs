pub mod grpc;

pub use ledger_api_types as types;

// Re-export
pub use canton_version;
pub use tonic;
pub use tower;

use canton_version::semver::{Comparator, Op, Prerelease, Version, VersionReq};

/// Returns a list of version requirements, which defines versions of the Canton node, compatible
/// with this client.
///
/// For Ledger API we follow the semver compatibility rules. E.g. if `canton-version` reports
/// `3.5.12`, then any version `^3.5.12` (`>=3.5.12 AND <4.0.0`) is considered compatible.
pub fn supported_versions() -> Vec<VersionReq> {
    let version = canton_version::canton_version();
    // If any pre-release version is considered compatible, it needs to be added here manually.
    // Otherwise pre-release versions are rejected.
    vec![VersionReq {
        comparators: vec![Comparator {
            op: Op::Caret,
            major: version.major,
            minor: Some(version.minor),
            patch: Some(version.patch),
            pre: Prerelease::EMPTY,
        }],
    }]
}

/// Check given version against [`supported_versions()`] and return `true`, if it's supported.
pub fn is_supported_version(version: &Version) -> bool {
    supported_versions().iter().any(|req| req.matches(version))
}
