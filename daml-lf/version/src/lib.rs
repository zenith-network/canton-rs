//! Daml LF version types

#![no_std]

use core::{cmp::Ordering, fmt};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Version {
    pub major: u32,
    pub minor: MinorVersion,
}

impl Version {
    /// Returns `true` if self version is compatible with other
    ///
    /// That means if you can decode a package of version `other` and you receive a package of
    /// version `self`, you will be able to decode it.
    pub fn compatible_with(&self, other: &Self) -> bool {
        if self.major != other.major {
            return false;
        }

        todo!()
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            ordering => ordering,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.major.fmt(f)?;
        f.write_str(".")?;
        self.minor.fmt(f)
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MinorVersion {
    Stable { version: u32 },
    Staging { version: u32, revision: u32 },
    Dev,
}

impl PartialEq for MinorVersion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Stable { version: l_version }, Self::Stable { version: r_version }) => {
                l_version == r_version
            }
            (
                Self::Staging {
                    version: l_version,
                    revision: l_revision,
                },
                Self::Staging {
                    version: r_version,
                    revision: r_revision,
                },
            ) => l_version == r_version && l_revision == r_revision,
            (Self::Dev, Self::Dev) => true,
            _ => false,
        }
    }
}

impl Eq for MinorVersion {}

impl PartialOrd for MinorVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MinorVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // dev version is always the latest
            (Self::Dev, Self::Dev) => Ordering::Equal,
            (Self::Dev, _) => Ordering::Greater,
            (_, Self::Dev) => Ordering::Less,

            (
                Self::Staging {
                    version: l_version,
                    revision: l_revision,
                },
                Self::Staging {
                    version: r_version,
                    revision: r_revision,
                },
            ) => match l_version.cmp(r_version) {
                Ordering::Equal => l_revision.cmp(r_revision),
                ordering => ordering,
            },

            (Self::Staging { .. }, _) => Ordering::Greater,
            (_, Self::Staging { .. }) => Ordering::Less,
            _ => todo!(),
        }
    }
}

impl fmt::Display for MinorVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MinorVersion::Stable { version } => version.fmt(f),
            MinorVersion::Staging { version, revision } => {
                write!(f, "{version}-rc{revision}")
            }
            MinorVersion::Dev => f.write_str("dev"),
        }
    }
}
