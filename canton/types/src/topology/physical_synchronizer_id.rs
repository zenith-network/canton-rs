use std::{borrow::Cow, fmt, num::ParseIntError, str::FromStr};

use crate::topology::{
    Namespace, ProtocolVersion, SynchronizerId, UniqueIdentifier,
    errors::{ProtocolVersionError, SynchronizerIdError},
};

/// A physical synchronizer ID consisting of a logical ID, serial and protocol version.
///
/// String representation: `<identifier>::<namespace>::<protocol-version>-<serial>`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalSynchronizerId {
    logical: SynchronizerId,
    serial: u32,
    protocol_version: ProtocolVersion,
}

impl PhysicalSynchronizerId {
    /// Delimiter between the logical ID and the suffix
    pub const PRIMARY_DELIM: &str = UniqueIdentifier::DELIMITER;

    /// Delimiter of the suffix part
    pub const SECONDARY_DELIM: &str = "-";

    /// Largest serial representable by Canton
    pub const MAX_SERIAL: u32 = i32::MAX as u32;

    /// Create a new physical synchronizer ID.
    ///
    /// The serial must not exceed [`Self::MAX_SERIAL`].
    pub fn new(
        logical: SynchronizerId,
        serial: u32,
        protocol_version: ProtocolVersion,
    ) -> Result<Self, PhysicalSynchronizerIdError> {
        if serial > Self::MAX_SERIAL {
            return Err(PhysicalSynchronizerIdError {
                kind: ErrorKind::SerialOutOfBounds { serial },
            });
        }
        Ok(Self {
            logical,
            serial,
            protocol_version,
        })
    }

    /// Logical synchronizer ID
    pub fn logical(&self) -> &SynchronizerId {
        &self.logical
    }

    /// Underlying logical unique identifier
    pub fn logical_uid(&self) -> &UniqueIdentifier {
        self.logical.uid()
    }

    pub fn identifier(&self) -> &str {
        self.logical.identifier()
    }

    pub fn namespace(&self) -> &Namespace {
        self.logical.namespace()
    }

    pub fn serial(&self) -> u32 {
        self.serial
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    /// Protocol version and serial separated by [`Self::SECONDARY_DELIM`]
    pub fn suffix(&self) -> String {
        format!(
            "{}{}{}",
            self.protocol_version,
            Self::SECONDARY_DELIM,
            self.serial
        )
    }

    /// Return a new ID with the serial incremented, or an error at [`Self::MAX_SERIAL`].
    pub fn increment_serial(&self) -> Result<Self, PhysicalSynchronizerIdError> {
        Self::new(self.logical.clone(), self.serial + 1, self.protocol_version)
    }

    /// Parse physical synchronizer ID from string.
    pub fn parse(s: &str) -> Result<Self, PhysicalSynchronizerIdError> {
        // Canton separates the suffix at the second delimiter from the left.
        let (suffix_index, _) =
            s.match_indices(Self::PRIMARY_DELIM)
                .nth(1)
                .ok_or(PhysicalSynchronizerIdError {
                    kind: ErrorKind::MissingSuffix,
                })?;
        let (logical, suffix) = s.split_at(suffix_index);
        let suffix = &suffix[Self::PRIMARY_DELIM.len()..];
        let logical =
            SynchronizerId::parse(logical).map_err(|err| PhysicalSynchronizerIdError {
                kind: ErrorKind::SynchronizerIdError(err),
            })?;
        let (protocol_version, serial) =
            suffix
                .rsplit_once(Self::SECONDARY_DELIM)
                .ok_or(PhysicalSynchronizerIdError {
                    kind: ErrorKind::InvalidSuffix,
                })?;
        let protocol_version = ProtocolVersion::parse(protocol_version)
            .map_err(|error| PhysicalSynchronizerIdError { kind: error.into() })?;
        let serial = serial
            .parse()
            .map_err(|source| PhysicalSynchronizerIdError {
                kind: ErrorKind::Serial { source },
            })?;
        Self::new(logical, serial, protocol_version)
    }
}

impl fmt::Display for PhysicalSynchronizerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}{}",
            self.logical,
            Self::PRIMARY_DELIM,
            self.protocol_version,
            Self::SECONDARY_DELIM,
            self.serial
        )
    }
}

impl FromStr for PhysicalSynchronizerId {
    type Err = PhysicalSynchronizerIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq<str> for PhysicalSynchronizerId {
    fn eq(&self, other: &str) -> bool {
        other
            .rsplit_once(Self::PRIMARY_DELIM)
            .is_some_and(|(logical, suffix)| self.logical.eq(logical) && suffix == self.suffix())
    }
}

impl PartialEq<&str> for PhysicalSynchronizerId {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<String> for PhysicalSynchronizerId {
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<Cow<'_, str>> for PhysicalSynchronizerId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.eq(other.as_ref())
    }
}

impl TryFrom<String> for PhysicalSynchronizerId {
    type Error = PhysicalSynchronizerIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<PhysicalSynchronizerId> for String {
    fn from(value: PhysicalSynchronizerId) -> Self {
        value.to_string()
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub struct PhysicalSynchronizerIdError {
    kind: ErrorKind,
}

#[derive(Clone, Debug, thiserror::Error)]
enum ErrorKind {
    #[error("physical synchronizer ID is missing a suffix")]
    MissingSuffix,

    #[error(
        "physical synchronizer ID suffix must contain a protocol version and serial separated by '-'"
    )]
    InvalidSuffix,

    #[error(transparent)]
    SynchronizerIdError(#[from] SynchronizerIdError),

    #[error(transparent)]
    ProtocolVersion(#[from] ProtocolVersionError),

    #[error("invalid serial: {source}")]
    Serial {
        #[source]
        source: ParseIntError,
    },

    #[error(
        "serial is out of bounds (got {serial}, max: {})",
        PhysicalSynchronizerId::MAX_SERIAL
    )]
    SerialOutOfBounds { serial: u32 },
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::topology::{Namespace, ProtocolVersion, SynchronizerId};

    use super::PhysicalSynchronizerId;

    #[rstest]
    #[case(0, ProtocolVersion::new_unchecked(34))]
    #[case(1, ProtocolVersion::new_unchecked(35))]
    #[case(PhysicalSynchronizerId::MAX_SERIAL, ProtocolVersion::new_unchecked(34))]
    #[should_panic(expected = "serial is out of bounds")]
    #[case(PhysicalSynchronizerId::MAX_SERIAL + 1, ProtocolVersion::new_unchecked(34))]
    #[should_panic(expected = "serial is out of bounds")]
    #[case(u32::MAX, ProtocolVersion::new_unchecked(34))]
    fn test_new(#[case] serial: u32, #[case] protocol_version: ProtocolVersion) {
        let logical = SynchronizerId::new(
            "synchronizer".to_owned(),
            Namespace::new("namespace".to_owned()).unwrap(),
        )
        .unwrap();
        let synchronizer = PhysicalSynchronizerId::new(logical.clone(), serial, protocol_version)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(synchronizer.logical.uid().identifier, "synchronizer");
        assert_eq!(synchronizer.logical.uid().namespace.as_str(), "namespace");
        assert_eq!(synchronizer.serial, serial);
        assert_eq!(synchronizer.protocol_version, protocol_version);
    }

    #[rstest]
    #[case(
        "synchronizer::namespace::34-0",
        "synchronizer",
        "namespace",
        0,
        ProtocolVersion::new_unchecked(34)
    )]
    #[case(
        "da-second::default::35-1",
        "da-second",
        "default",
        1,
        ProtocolVersion::new_unchecked(35)
    )]
    #[should_panic(expected = "protocol version error")]
    #[case(":id::namespace:::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[case(
        "id:::namespace::34-0",
        "id",
        ":namespace",
        0,
        ProtocolVersion::new_unchecked(34)
    )]
    #[case(" :: ::34-0", " ", " ", 0, ProtocolVersion::new_unchecked(34))]
    #[case(
        "foo:bar-baz_9::one:two::34-0",
        "foo:bar-baz_9",
        "one:two",
        0,
        ProtocolVersion::new_unchecked(34)
    )]
    #[case("id::ns::34-+0001", "id", "ns", 1, ProtocolVersion::new_unchecked(34))]
    #[case(
        "a".repeat(185) + "::" + &"b".repeat(68) + "::34-2147483647",
        "a".repeat(185),
        "b".repeat(68),
        PhysicalSynchronizerId::MAX_SERIAL,
        ProtocolVersion::new_unchecked(34)
    )]
    #[should_panic(expected = "missing a suffix")]
    #[case("", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "missing a suffix")]
    #[case("synchronizer", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "missing a suffix")]
    #[case(
        "synchronizer::namespace",
        "",
        "",
        0,
        ProtocolVersion::new_unchecked(0)
    )]
    #[should_panic(expected = "empty first part")]
    #[case("::namespace::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "namespace is empty")]
    #[case("synchronizer::::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "unexpected character '%' in unique identifier")]
    #[case("bad%::namespace::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("synchronizer::ਊ::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "protocol version error")]
    #[case(
        "id::namespace::extra::34-0",
        "",
        "",
        0,
        ProtocolVersion::new_unchecked(0)
    )]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("a".repeat(186) + "::namespace::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("id::".to_owned() + &"b".repeat(69) + "::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "suffix must contain a protocol version and serial")]
    #[case("id::namespace::", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "suffix must contain a protocol version and serial")]
    #[case("id::namespace::34", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-bad", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "invalid serial")]
    #[case(
        "id::namespace::34-4294967296",
        "",
        "",
        0,
        ProtocolVersion::new_unchecked(0)
    )]
    #[should_panic(expected = "serial is out of bounds")]
    #[case(
        "id::namespace::34-2147483648",
        "",
        "",
        0,
        ProtocolVersion::new_unchecked(0)
    )]
    fn test_parse(
        #[case] input: String,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
        #[case] expected_serial: u32,
        #[case] expected_protocol_version: ProtocolVersion,
    ) {
        let synchronizer =
            PhysicalSynchronizerId::parse(&input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(synchronizer.logical.uid().identifier, expected_identifier);
        assert_eq!(
            synchronizer.logical.uid().namespace.as_str(),
            expected_namespace
        );
        assert_eq!(synchronizer.serial, expected_serial);
        assert_eq!(synchronizer.protocol_version, expected_protocol_version);
    }

    #[rstest]
    #[case(
        "synchronizer::namespace::34-0",
        "synchronizer",
        "namespace",
        0,
        ProtocolVersion::new_unchecked(34)
    )]
    #[case(
        "da-second::default::35-1",
        "da-second",
        "default",
        1,
        ProtocolVersion::new_unchecked(35)
    )]
    #[should_panic(expected = "protocol version error")]
    #[case(":id::namespace:::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[case(
        "id:::namespace::34-0",
        "id",
        ":namespace",
        0,
        ProtocolVersion::new_unchecked(34)
    )]
    #[case(" :: ::34-0", " ", " ", 0, ProtocolVersion::new_unchecked(34))]
    #[case(
        "foo:bar-baz_9::one:two::34-0",
        "foo:bar-baz_9",
        "one:two",
        0,
        ProtocolVersion::new_unchecked(34)
    )]
    #[case("id::ns::34-+0001", "id", "ns", 1, ProtocolVersion::new_unchecked(34))]
    #[case(
        "a".repeat(185) + "::" + &"b".repeat(68) + "::34-2147483647",
        "a".repeat(185),
        "b".repeat(68),
        PhysicalSynchronizerId::MAX_SERIAL,
        ProtocolVersion::new_unchecked(34)
    )]
    #[should_panic(expected = "missing a suffix")]
    #[case("", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "missing a suffix")]
    #[case("synchronizer", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "missing a suffix")]
    #[case(
        "synchronizer::namespace",
        "",
        "",
        0,
        ProtocolVersion::new_unchecked(0)
    )]
    #[should_panic(expected = "empty first part")]
    #[case("::namespace::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "namespace is empty")]
    #[case("synchronizer::::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "unexpected character '%' in unique identifier")]
    #[case("bad%::namespace::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("synchronizer::ਊ::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "protocol version error")]
    #[case(
        "id::namespace::extra::34-0",
        "",
        "",
        0,
        ProtocolVersion::new_unchecked(0)
    )]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("a".repeat(186) + "::namespace::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("id::".to_owned() + &"b".repeat(69) + "::34-0", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "suffix must contain a protocol version and serial")]
    #[case("id::namespace::", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "suffix must contain a protocol version and serial")]
    #[case("id::namespace::34", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-bad", "", "", 0, ProtocolVersion::new_unchecked(0))]
    #[should_panic(expected = "invalid serial")]
    #[case(
        "id::namespace::34-4294967296",
        "",
        "",
        0,
        ProtocolVersion::new_unchecked(0)
    )]
    #[should_panic(expected = "serial is out of bounds")]
    #[case(
        "id::namespace::34-2147483648",
        "",
        "",
        0,
        ProtocolVersion::new_unchecked(0)
    )]
    fn test_try_from_string(
        #[case] input: String,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
        #[case] expected_serial: u32,
        #[case] expected_protocol_version: ProtocolVersion,
    ) {
        let synchronizer =
            PhysicalSynchronizerId::try_from(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(synchronizer.logical.uid().identifier, expected_identifier);
        assert_eq!(
            synchronizer.logical.uid().namespace.as_str(),
            expected_namespace
        );
        assert_eq!(synchronizer.serial, expected_serial);
        assert_eq!(synchronizer.protocol_version, expected_protocol_version);
    }

    #[rstest]
    #[case(
        "synchronizer",
        "namespace",
        0,
        ProtocolVersion::new_unchecked(34),
        "synchronizer::namespace::34-0"
    )]
    #[case(
        "da-second",
        "default",
        1,
        ProtocolVersion::new_unchecked(35),
        "da-second::default::35-1"
    )]
    #[case(
        ":id",
        "namespace:",
        0,
        ProtocolVersion::new_unchecked(34),
        ":id::namespace:::34-0"
    )]
    #[case(
        "id:",
        "namespace",
        0,
        ProtocolVersion::new_unchecked(34),
        "id:::namespace::34-0"
    )]
    #[case(
        "id",
        ":namespace",
        0,
        ProtocolVersion::new_unchecked(34),
        "id:::namespace::34-0"
    )]
    #[case(" ", " ", 0, ProtocolVersion::new_unchecked(34), " :: ::34-0")]
    #[case(
        "foo:bar-baz_9",
        "one:two",
        0,
        ProtocolVersion::new_unchecked(34),
        "foo:bar-baz_9::one:two::34-0"
    )]
    #[case(
        "a".repeat(185),
        "b".repeat(68),
        PhysicalSynchronizerId::MAX_SERIAL,
        ProtocolVersion::new_unchecked(34),
        "a".repeat(185) + "::" + &"b".repeat(68) + "::34-2147483647"
    )]
    fn test_into_string(
        #[case] identifier: String,
        #[case] namespace: String,
        #[case] serial: u32,
        #[case] protocol_version: ProtocolVersion,
        #[case] expected: String,
    ) {
        let synchronizer = PhysicalSynchronizerId {
            logical: SynchronizerId::new(identifier, Namespace::new(namespace).unwrap()).unwrap(),
            serial,
            protocol_version,
        };

        let actual: String = synchronizer.into();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::34-0",
        true
    )]
    #[case(
        "id:",
        "ns",
        1,
        ProtocolVersion::new_unchecked(35),
        "id:::ns::35-1",
        true
    )]
    #[case(
        "id",
        ":ns",
        1,
        ProtocolVersion::new_unchecked(35),
        "id:::ns::35-1",
        true
    )]
    #[case(
        "id",
        "ns:",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns:::34-0",
        true
    )]
    #[case(" ", " ", 0, ProtocolVersion::new_unchecked(34), " :: ::34-0", true)]
    #[case(
        "id",
        "ns",
        PhysicalSynchronizerId::MAX_SERIAL,
        ProtocolVersion::new_unchecked(34),
        "id::ns::34-2147483647",
        true
    )]
    #[case("id", "ns", 0, ProtocolVersion::new_unchecked(34), "", false)]
    #[case("id", "ns", 0, ProtocolVersion::new_unchecked(34), "id::ns", false)]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "other::ns::34-0",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::other::34-0",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::35-0",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::34-1",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::034-0",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::34-00",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::+34-0",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::34-+0",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::34-0::extra",
        false
    )]
    #[case(
        "id",
        "ns",
        0,
        ProtocolVersion::new_unchecked(34),
        "id::ns::34-0 ",
        false
    )]
    fn test_eq_string(
        #[case] identifier: String,
        #[case] namespace: String,
        #[case] serial: u32,
        #[case] protocol_version: ProtocolVersion,
        #[case] other: String,
        #[case] expected: bool,
    ) {
        let synchronizer = PhysicalSynchronizerId {
            logical: SynchronizerId::new(identifier, Namespace::new(namespace).unwrap()).unwrap(),
            serial,
            protocol_version,
        };

        assert_eq!(synchronizer == other, expected);
    }

    #[rstest]
    #[case(0, ProtocolVersion::new_unchecked(34), 1)]
    #[case(1, ProtocolVersion::new_unchecked(35), 2)]
    #[case(PhysicalSynchronizerId::MAX_SERIAL - 1, ProtocolVersion::new_unchecked(34), PhysicalSynchronizerId::MAX_SERIAL)]
    #[should_panic(expected = "serial is out of bounds")]
    #[case(
        PhysicalSynchronizerId::MAX_SERIAL,
        ProtocolVersion::new_unchecked(34),
        0
    )]
    fn test_increment_serial(
        #[case] serial: u32,
        #[case] protocol_version: ProtocolVersion,
        #[case] expected_serial: u32,
    ) {
        let synchronizer = PhysicalSynchronizerId {
            logical: SynchronizerId::new(
                "synchronizer".to_owned(),
                Namespace::new("namespace".to_owned()).unwrap(),
            )
            .unwrap(),
            serial,
            protocol_version,
        };

        let incremented = synchronizer
            .increment_serial()
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(incremented.logical.uid().identifier, "synchronizer");
        assert_eq!(incremented.logical.uid().namespace.as_str(), "namespace");
        assert_eq!(incremented.serial, expected_serial);
        assert_eq!(incremented.protocol_version, protocol_version);
    }
}
