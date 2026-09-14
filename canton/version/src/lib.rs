//! The goal of this crate is to pin the Canton version, which was used to build the definitions.
//!
//! `CANTON_VERSION` is a one-line file, which contains the version of Canton and parsed here to be
//! a valid SemVer version. To update the Canton version, update the content of `CANTON_VERSION`
//! file.

// Re-export
pub use semver;

use semver::Version;
use std::sync::LazyLock;

/// This version is a version of Canton release (version of the node binary).
/// It implies that the Protobuf definitions are used from this checkout.
/// Because of that this needs to be aligned with the `upstream.toml`.
pub const CANTON_VERSION: LazyLock<Version> =
    LazyLock::new(|| match Version::parse(env!("CANTON_VERSION")) {
        Ok(version) => version,
        Err(_) => unreachable!("CANTON_VERSION must be verified by build script"),
    });
