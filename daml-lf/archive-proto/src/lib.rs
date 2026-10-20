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
        minor: daml_lf_version::MinorVersion::Stable { version: 4 },
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

#[cfg(test)]
mod tests {
    use daml_lf_version::{MinorVersion, Version};

    use super::is_supported;

    #[cfg(feature = "v2")]
    #[test]
    fn external_call_builtin_has_stable_and_legacy_dev_ids() {
        use crate::com::digitalasset::daml::lf::archive::v2::BuiltinFunction;

        // Stable LF 2.4 (Canton 3.6.1, damlc 3.6.2).
        assert_eq!(
            BuiltinFunction::try_from(74),
            Ok(BuiltinFunction::ExternalCall)
        );
        // Archives from earlier 2.dev compilers.
        assert_eq!(
            BuiltinFunction::try_from(5001),
            Ok(BuiltinFunction::ExternalCallLegacyDev)
        );
    }

    #[cfg(feature = "v2")]
    #[test]
    fn supports_stable_lf_2_4_but_not_2_5() {
        let stable = |version| Version {
            major: 2,
            minor: MinorVersion::Stable { version },
        };
        assert!(is_supported(&stable(4)));
        assert!(!is_supported(&stable(5)));
        assert!(!is_supported(&Version {
            major: 2,
            minor: MinorVersion::Staging {
                version: 4,
                revision: 2,
            },
        }));
    }
}
