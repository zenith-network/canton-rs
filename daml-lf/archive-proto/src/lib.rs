//! Daml LF Protobuf

pub use daml_lf_version;
pub use prost;

use daml_lf_version::Version;

pub mod com {
    pub mod digitalasset {
        pub mod daml {
            pub mod lf {
                pub mod archive {
                    include!(concat!(env!("OUT_DIR"), "/daml_lf.rs"));

                    #[cfg(feature = "v2")]
                    pub mod v2 {
                        include!(concat!(env!("OUT_DIR"), "/daml_lf_2.rs"));
                    }
                }
            }
        }
    }
}

/// List of supported Daml LF versions.
///
/// This list is maintained manually based on the content of compiled Protobuf code.
pub const SUPPORTED_VERSIONS: &[Version] = &[
    #[cfg(feature = "v2")]
    Version {
        major: 2,
        minor: daml_lf_version::MinorVersion::Stable { version: 1 },
    },
    #[cfg(feature = "v2")]
    Version {
        major: 2,
        minor: daml_lf_version::MinorVersion::Stable { version: 2 },
    },
    #[cfg(feature = "v2")]
    Version {
        major: 2,
        minor: daml_lf_version::MinorVersion::Stable { version: 3 },
    },
    #[cfg(feature = "v2")]
    Version {
        major: 2,
        minor: daml_lf_version::MinorVersion::Staging {
            version: 4,
            revision: 1,
        },
    },
    #[cfg(feature = "v2")]
    Version {
        major: 2,
        minor: daml_lf_version::MinorVersion::Dev,
    },
];

pub fn is_supported(version: &Version) -> bool {
    SUPPORTED_VERSIONS.iter().find(|v| v == &version).is_some()
}
