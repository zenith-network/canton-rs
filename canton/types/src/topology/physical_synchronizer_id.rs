use std::{borrow::Cow, fmt, num::ParseIntError, str::FromStr};

use crate::topology::{
    Identifier, Namespace, ProtocolVersion, SynchronizerId, errors::IdentifierError,
};

/// A physical synchronizer ID consisting of a logical ID, serial and protocol version.
///
/// String representation: `identifier::namespace::protocol-version-serial`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalSynchronizerId {
    logical: SynchronizerId,
    serial: u32,
    protocol_version: ProtocolVersion,
}

impl PhysicalSynchronizerId {
    /// Delimiter between the logical ID and the suffix
    pub const PRIMARY_DELIM: &str = Identifier::DELIMITER;

    /// Delimiter of the suffix part
    pub const SECONDARY_DELIM: &str = "-";

    /// Largest serial representable by Canton's non-negative signed 32-bit integer
    pub const MAX_SERIAL: u32 = i32::MAX as u32;

    /// Create a new physical synchronizer ID.
    ///
    /// The serial must not exceed [`Self::MAX_SERIAL`]. Protocol versions are
    /// accepted without checking whether Canton supports them.
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

    /// Underlying logical identifier
    pub fn id(&self) -> &Identifier {
        self.logical.id()
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

    pub fn parse(s: &str) -> Result<Self, PhysicalSynchronizerIdError> {
        let (logical, suffix) =
            s.rsplit_once(Self::PRIMARY_DELIM)
                .ok_or(PhysicalSynchronizerIdError {
                    kind: ErrorKind::MissingSuffix,
                })?;
        let logical =
            SynchronizerId::parse(logical).map_err(|err| PhysicalSynchronizerIdError {
                kind: ErrorKind::Identifier(err),
            })?;
        let (protocol_version, serial) =
            suffix
                .rsplit_once(Self::SECONDARY_DELIM)
                .ok_or(PhysicalSynchronizerIdError {
                    kind: ErrorKind::InvalidSuffix,
                })?;
        let protocol_version = ProtocolVersion::parse(protocol_version).map_err(|source| {
            PhysicalSynchronizerIdError {
                kind: ErrorKind::ProtocolVersion { source },
            }
        })?;
        let serial = serial
            .parse()
            .map_err(|source| PhysicalSynchronizerIdError {
                kind: ErrorKind::Serial { source },
            })?;
        Self::new(logical, serial, protocol_version)
    }
}

impl FromStr for PhysicalSynchronizerId {
    type Err = PhysicalSynchronizerIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for PhysicalSynchronizerId {
    type Error = PhysicalSynchronizerIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl TryFrom<&'_ str> for PhysicalSynchronizerId {
    type Error = PhysicalSynchronizerIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
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

impl From<PhysicalSynchronizerId> for String {
    fn from(value: PhysicalSynchronizerId) -> Self {
        value.to_string()
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

impl PartialEq<PhysicalSynchronizerId> for str {
    fn eq(&self, other: &PhysicalSynchronizerId) -> bool {
        other.eq(self)
    }
}

impl PartialEq<PhysicalSynchronizerId> for &str {
    fn eq(&self, other: &PhysicalSynchronizerId) -> bool {
        other.eq(*self)
    }
}

impl PartialEq<PhysicalSynchronizerId> for String {
    fn eq(&self, other: &PhysicalSynchronizerId) -> bool {
        other.eq(self.as_str())
    }
}

impl PartialEq<PhysicalSynchronizerId> for Cow<'_, str> {
    fn eq(&self, other: &PhysicalSynchronizerId) -> bool {
        other.eq(self.as_ref())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct PhysicalSynchronizerIdError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("physical synchronizer ID is missing a suffix")]
    MissingSuffix,

    #[error(
        "physical synchronizer ID suffix must contain a protocol version and serial separated by '-'"
    )]
    InvalidSuffix,

    #[error(transparent)]
    Identifier(#[from] IdentifierError),

    #[error("invalid protocol version: {source}")]
    ProtocolVersion {
        #[source]
        source: ParseIntError,
    },

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
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(0, 34)]
    #[case(1, 35)]
    #[case(PhysicalSynchronizerId::MAX_SERIAL, i32::MAX)]
    #[case(0, 0)]
    #[case(0, -1)]
    #[case(0, i32::MIN)]
    #[should_panic(expected = "serial is out of bounds")]
    #[case(PhysicalSynchronizerId::MAX_SERIAL + 1, 34)]
    #[should_panic(expected = "serial is out of bounds")]
    #[case(u32::MAX, 34)]
    fn test_new(#[case] serial: u32, #[case] protocol_version: i32) {
        let logical = SynchronizerId::new(
            "synchronizer".to_owned(),
            Namespace::new("namespace".to_owned()).unwrap(),
        )
        .unwrap();
        let synchronizer = PhysicalSynchronizerId::new(
            logical.clone(),
            serial,
            ProtocolVersion::new(protocol_version),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(synchronizer.logical.id().identifier, "synchronizer");
        assert_eq!(synchronizer.logical.id().namespace.as_str(), "namespace");
        assert_eq!(synchronizer.serial, serial);
        assert_eq!(i32::from(synchronizer.protocol_version), protocol_version);
    }

    #[rstest]
    #[case("synchronizer::namespace::34-0", "synchronizer", "namespace", 0, 34)]
    #[case("da-second::default::35-1", "da-second", "default", 1, 35)]
    #[case(":id::namespace:::34-0", ":id", "namespace:", 0, 34)]
    #[case("id:::namespace::34-0", "id", ":namespace", 0, 34)]
    #[case(" :: ::34-0", " ", " ", 0, 34)]
    #[case("foo:bar-baz_9::one:two::34-0", "foo:bar-baz_9", "one:two", 0, 34)]
    #[case("id::ns::+0034-+0001", "id", "ns", 1, 34)]
    #[case("id::ns::0-0", "id", "ns", 0, 0)]
    #[case("id::ns::-1-0", "id", "ns", 0, -1)]
    #[case("id::ns::-2147483648-0", "id", "ns", 0, i32::MIN)]
    #[case(
        "a".repeat(185) + "::" + &"b".repeat(68) + "::2147483647-2147483647",
        "a".repeat(185),
        "b".repeat(68),
        PhysicalSynchronizerId::MAX_SERIAL,
        i32::MAX
    )]
    #[should_panic(expected = "missing a suffix")]
    #[case("", "", "", 0, 0)]
    #[should_panic(expected = "missing a suffix")]
    #[case("synchronizer", "", "", 0, 0)]
    #[should_panic(expected = "missing a namespace")]
    #[case("synchronizer::namespace", "", "", 0, 0)]
    #[should_panic(expected = "empty first part")]
    #[case("::namespace::34-0", "", "", 0, 0)]
    #[should_panic(expected = "namespace is empty")]
    #[case("synchronizer::::34-0", "", "", 0, 0)]
    #[should_panic(expected = "unexpected character '%' in identifier")]
    #[case("bad%::namespace::34-0", "", "", 0, 0)]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("synchronizer::ਊ::34-0", "", "", 0, 0)]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case("id::namespace::extra::34-0", "", "", 0, 0)]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("a".repeat(186) + "::namespace::34-0", "", "", 0, 0)]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("id::".to_owned() + &"b".repeat(69) + "::34-0", "", "", 0, 0)]
    #[should_panic(expected = "suffix must contain a protocol version and serial")]
    #[case("id::namespace::", "", "", 0, 0)]
    #[should_panic(expected = "suffix must contain a protocol version and serial")]
    #[case("id::namespace::34", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::bad-0", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::-0", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::2147483648-0", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::34--1", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::34-0-1", "", "", 0, 0)]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-", "", "", 0, 0)]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-bad", "", "", 0, 0)]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-4294967296", "", "", 0, 0)]
    #[should_panic(expected = "serial is out of bounds")]
    #[case("id::namespace::34-2147483648", "", "", 0, 0)]
    fn test_parse(
        #[case] input: String,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
        #[case] expected_serial: u32,
        #[case] expected_protocol_version: i32,
    ) {
        let synchronizer =
            PhysicalSynchronizerId::parse(&input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(synchronizer.logical.id().identifier, expected_identifier);
        assert_eq!(
            synchronizer.logical.id().namespace.as_str(),
            expected_namespace
        );
        assert_eq!(synchronizer.serial, expected_serial);
        assert_eq!(
            i32::from(synchronizer.protocol_version),
            expected_protocol_version
        );
    }

    #[rstest]
    #[case("synchronizer::namespace::34-0", "synchronizer", "namespace", 0, 34)]
    #[case("da-second::default::35-1", "da-second", "default", 1, 35)]
    #[case(":id::namespace:::34-0", ":id", "namespace:", 0, 34)]
    #[case("id:::namespace::34-0", "id", ":namespace", 0, 34)]
    #[case(" :: ::34-0", " ", " ", 0, 34)]
    #[case("foo:bar-baz_9::one:two::34-0", "foo:bar-baz_9", "one:two", 0, 34)]
    #[case("id::ns::+0034-+0001", "id", "ns", 1, 34)]
    #[case("id::ns::0-0", "id", "ns", 0, 0)]
    #[case("id::ns::-1-0", "id", "ns", 0, -1)]
    #[case("id::ns::-2147483648-0", "id", "ns", 0, i32::MIN)]
    #[case(
        "a".repeat(185) + "::" + &"b".repeat(68) + "::2147483647-2147483647",
        "a".repeat(185),
        "b".repeat(68),
        PhysicalSynchronizerId::MAX_SERIAL,
        i32::MAX
    )]
    #[should_panic(expected = "missing a suffix")]
    #[case("", "", "", 0, 0)]
    #[should_panic(expected = "missing a suffix")]
    #[case("synchronizer", "", "", 0, 0)]
    #[should_panic(expected = "missing a namespace")]
    #[case("synchronizer::namespace", "", "", 0, 0)]
    #[should_panic(expected = "empty first part")]
    #[case("::namespace::34-0", "", "", 0, 0)]
    #[should_panic(expected = "namespace is empty")]
    #[case("synchronizer::::34-0", "", "", 0, 0)]
    #[should_panic(expected = "unexpected character '%' in identifier")]
    #[case("bad%::namespace::34-0", "", "", 0, 0)]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("synchronizer::ਊ::34-0", "", "", 0, 0)]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case("id::namespace::extra::34-0", "", "", 0, 0)]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("a".repeat(186) + "::namespace::34-0", "", "", 0, 0)]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("id::".to_owned() + &"b".repeat(69) + "::34-0", "", "", 0, 0)]
    #[should_panic(expected = "suffix must contain a protocol version and serial")]
    #[case("id::namespace::", "", "", 0, 0)]
    #[should_panic(expected = "suffix must contain a protocol version and serial")]
    #[case("id::namespace::34", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::bad-0", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::-0", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::2147483648-0", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::34--1", "", "", 0, 0)]
    #[should_panic(expected = "invalid protocol version")]
    #[case("id::namespace::34-0-1", "", "", 0, 0)]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-", "", "", 0, 0)]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-bad", "", "", 0, 0)]
    #[should_panic(expected = "invalid serial")]
    #[case("id::namespace::34-4294967296", "", "", 0, 0)]
    #[should_panic(expected = "serial is out of bounds")]
    #[case("id::namespace::34-2147483648", "", "", 0, 0)]
    fn test_try_from_string(
        #[case] input: String,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
        #[case] expected_serial: u32,
        #[case] expected_protocol_version: i32,
    ) {
        let synchronizer =
            PhysicalSynchronizerId::try_from(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(synchronizer.logical.id().identifier, expected_identifier);
        assert_eq!(
            synchronizer.logical.id().namespace.as_str(),
            expected_namespace
        );
        assert_eq!(synchronizer.serial, expected_serial);
        assert_eq!(
            i32::from(synchronizer.protocol_version),
            expected_protocol_version
        );
    }

    #[rstest]
    #[case("synchronizer", "namespace", 0, 34, "synchronizer::namespace::34-0")]
    #[case("da-second", "default", 1, 35, "da-second::default::35-1")]
    #[case(":id", "namespace:", 0, 34, ":id::namespace:::34-0")]
    #[case("id:", "namespace", 0, 34, "id:::namespace::34-0")]
    #[case("id", ":namespace", 0, 34, "id:::namespace::34-0")]
    #[case(" ", " ", 0, 34, " :: ::34-0")]
    #[case("foo:bar-baz_9", "one:two", 0, 34, "foo:bar-baz_9::one:two::34-0")]
    #[case("id", "ns", 0, 0, "id::ns::0-0")]
    #[case("id", "ns", 0, -1, "id::ns::-1-0")]
    #[case("id", "ns", 0, i32::MIN, "id::ns::-2147483648-0")]
    #[case(
        "a".repeat(185),
        "b".repeat(68),
        PhysicalSynchronizerId::MAX_SERIAL,
        i32::MAX,
        "a".repeat(185) + "::" + &"b".repeat(68) + "::2147483647-2147483647"
    )]
    fn test_into_string(
        #[case] identifier: String,
        #[case] namespace: String,
        #[case] serial: u32,
        #[case] protocol_version: i32,
        #[case] expected: String,
    ) {
        let synchronizer = PhysicalSynchronizerId {
            logical: SynchronizerId::new(identifier, Namespace::new(namespace).unwrap()).unwrap(),
            serial,
            protocol_version: ProtocolVersion::new(protocol_version),
        };

        let actual: String = synchronizer.into();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case("id", "ns", 0, 34, "id::ns::34-0", true)]
    #[case("id", "ns", 0, -1, "id::ns::-1-0", true)]
    #[case("id:", "ns", 1, 35, "id:::ns::35-1", true)]
    #[case("id", ":ns", 1, 35, "id:::ns::35-1", true)]
    #[case("id", "ns:", 0, 34, "id::ns:::34-0", true)]
    #[case(" ", " ", 0, 0, " :: ::0-0", true)]
    #[case(
        "id",
        "ns",
        PhysicalSynchronizerId::MAX_SERIAL,
        i32::MIN,
        "id::ns::-2147483648-2147483647",
        true
    )]
    #[case("id", "ns", 0, 34, "", false)]
    #[case("id", "ns", 0, 34, "id::ns", false)]
    #[case("id", "ns", 0, 34, "other::ns::34-0", false)]
    #[case("id", "ns", 0, 34, "id::other::34-0", false)]
    #[case("id", "ns", 0, 34, "id::ns::35-0", false)]
    #[case("id", "ns", 0, 34, "id::ns::34-1", false)]
    #[case("id", "ns", 0, 34, "id::ns::034-0", false)]
    #[case("id", "ns", 0, 34, "id::ns::34-00", false)]
    #[case("id", "ns", 0, 34, "id::ns::+34-0", false)]
    #[case("id", "ns", 0, 34, "id::ns::34-+0", false)]
    #[case("id", "ns", 0, 34, "id::ns::34-0::extra", false)]
    #[case("id", "ns", 0, 34, "id::ns::34-0 ", false)]
    fn test_eq_string(
        #[case] identifier: String,
        #[case] namespace: String,
        #[case] serial: u32,
        #[case] protocol_version: i32,
        #[case] other: String,
        #[case] expected: bool,
    ) {
        let synchronizer = PhysicalSynchronizerId {
            logical: SynchronizerId::new(identifier, Namespace::new(namespace).unwrap()).unwrap(),
            serial,
            protocol_version: ProtocolVersion::new(protocol_version),
        };

        assert_eq!(synchronizer == other, expected);
    }

    #[rstest]
    #[case(0, 34, 1)]
    #[case(1, 35, 2)]
    #[case(9, 0, 10)]
    #[case(99, -1, 100)]
    #[case(PhysicalSynchronizerId::MAX_SERIAL - 1, i32::MIN, PhysicalSynchronizerId::MAX_SERIAL)]
    #[should_panic(expected = "serial is out of bounds")]
    #[case(PhysicalSynchronizerId::MAX_SERIAL, 34, 0)]
    fn test_increment_serial(
        #[case] serial: u32,
        #[case] protocol_version: i32,
        #[case] expected_serial: u32,
    ) {
        let synchronizer = PhysicalSynchronizerId {
            logical: SynchronizerId::new(
                "synchronizer".to_owned(),
                Namespace::new("namespace".to_owned()).unwrap(),
            )
            .unwrap(),
            serial,
            protocol_version: ProtocolVersion::new(protocol_version),
        };

        let incremented = synchronizer
            .increment_serial()
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(incremented.logical.id().identifier, "synchronizer");
        assert_eq!(incremented.logical.id().namespace.as_str(), "namespace");
        assert_eq!(incremented.serial, expected_serial);
        assert_eq!(i32::from(incremented.protocol_version), protocol_version);
    }
}
