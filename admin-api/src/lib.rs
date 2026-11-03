use canton_version::semver::{Comparator, Op, Prerelease, Version, VersionReq};

#[cfg(feature = "v30")]
pub mod v30;

/// Returns a list of version requirements, which defines versions of the Canton node, compatible
/// with Admin API client.
pub fn supported_versions() -> Vec<VersionReq> {
    // For now I'm gonna put 3.5.x and 3.6.x versions here (optimistic).
    // We would have to probably maintain this list manually based on integration testing, since
    // Admin API currently doesn't have sem ver guarantees in Canton.
    vec![VersionReq {
        comparators: vec![
            Comparator {
                op: Op::GreaterEq,
                major: 3,
                minor: Some(5),
                patch: None,
                pre: Prerelease::EMPTY,
            },
            Comparator {
                op: Op::Less,
                major: 3,
                minor: Some(7),
                patch: None,
                pre: Prerelease::EMPTY,
            },
        ],
    }]
}

/// Check given version against [`supported_versions()`] and return `true`, if it's supported.
pub fn is_supported_version(version: &Version) -> bool {
    supported_versions().iter().any(|req| req.matches(version))
}
