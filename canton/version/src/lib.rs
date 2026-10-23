//! The goal of this crate is to pin the Canton version, which was used to build the definitions.
//!
//! `CANTON_VERSION` is a one-line file, which contains the version of Canton and parsed here to be
//! a valid SemVer version. To update the Canton version, update the content of `CANTON_VERSION`
//! file.

// Re-export
pub use semver;

use semver::Version;
use std::sync::LazyLock;

static CANTON_VERSION: LazyLock<Version> =
    LazyLock::new(|| match Version::parse(env!("CANTON_VERSION")) {
        Ok(version) => version,
        Err(_) => unreachable!("CANTON_VERSION must be verified by build script"),
    });

/// This version is a version of Canton release (version of the node binary).
/// It implies that the Protobuf definitions are used from this checkout.
/// Because of that this needs to be aligned with the `upstream.toml`.
pub fn canton_version() -> &'static Version {
    // Technically this version could be a base version for pinned revision from `upstream.toml`.
    // However the developer has to ensure, that it's compatible.
    // This would allow to re-pin capsules without a new Canton release.
    // For a stable SDK release these revisions should align though.
    LazyLock::force(&CANTON_VERSION)
}

/// Original string representation of [`canton_version()`].
pub fn canton_version_str() -> &'static str {
    env!("CANTON_VERSION")
}
