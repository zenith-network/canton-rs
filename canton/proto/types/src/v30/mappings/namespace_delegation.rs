use std::collections::BTreeSet;

use canton_crypto::v30::{EncodedSigningPublicKey, SigningPublicKeyError};
use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::topology::Namespace;
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue, RequiredProtoField as _};

use crate::{error::ConvertError, v30::mappings::TopologyMappingCode};

/// Namespace delegation (NSD)
///
/// Equivalent to X509v3 CA root or intermediate CAs.
///
/// ## Authorization
///
/// A namespace delegation is either signed by the root key, or is signed by a key for which there
/// exists a series of properly authorized namespace delegations that are ultimately signed by the
/// root key.
///
/// ## Revocation
///
/// A revoked namespace delegation cannot be re-created. While the delegation itself is revoked,
/// valid transactions that have been signed using the authority of the delegation before its
/// revocation stay valid.
///
/// ## Uniqueness key
///
/// ```plaintext
/// (namespace, target_key)
/// ```
///
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NamespaceDelegation {
    namespace: Namespace,
    target_key: EncodedSigningPublicKey,
    restriction: Restriction,
}

impl NamespaceDelegation {
    /// Create new namespace delegation
    pub fn new(
        namespace: Namespace,
        target_key: EncodedSigningPublicKey,
        restriction: Restriction,
    ) -> Result<Self, NsdError> {
        let nsd = Self {
            namespace,
            target_key,
            restriction,
        };
        // Otherwise NSD is rejected by Canton
        if nsd.is_root()? && !nsd.can_delegate() {
            return Err(NsdError::RootCantDelegate);
        }
        Ok(nsd)
    }

    /// Fingerprint of the root key defining the namespace
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Target key of getting full rights on the namespace
    ///
    /// If `target == namespace`, it's a root certificate. User can check that with
    /// [`Self::is_root`].
    pub fn target_key(&self) -> &EncodedSigningPublicKey {
        &self.target_key
    }

    /// Restricts `target_key` to only sign transactions with the specified mapping types.
    ///
    /// For backwards compatibility, only the following combinations are valid:
    ///
    /// - `is_root_delegation = true`, `restriction = empty`: the key can sign all mappings
    /// - `is_root_delegation = false`, `restriction = empty`: the key can sign all mappings but
    ///   namespace delegations
    /// - `is_root_delegation = false`, `restriction = non-empty`: the key can only sign the mappings
    ///   according the restriction that is set
    pub fn restriction(&self) -> &Restriction {
        &self.restriction
    }

    /// Returns `true` if `target == namespace`
    pub fn is_root(&self) -> Result<bool, SigningPublicKeyError> {
        let target_namespace = Namespace::from_fingerprint(self.target_key.fingerprint()?);
        Ok(target_namespace == self.namespace)
    }

    /// Returns `true` if the delegate can sign other namespace delegations.
    pub fn can_delegate(&self) -> bool {
        self.restriction.can_delegate()
    }
}

impl From<NamespaceDelegation> for proto::NamespaceDelegation {
    fn from(value: NamespaceDelegation) -> Self {
        Self {
            namespace: value.namespace.into(),
            target_key: Some(value.target_key.into()),
            #[allow(deprecated)]
            is_root_delegation: false,
            restriction: Some(value.restriction.into()),
        }
    }
}

impl TryFrom<proto::NamespaceDelegation> for NamespaceDelegation {
    type Error = ConvertError;

    fn try_from(value: proto::NamespaceDelegation) -> Result<Self, Self::Error> {
        let namespace = Namespace::new(value.namespace)
            .validated_of::<proto::NamespaceDelegation>("namespace")?;
        let target_key = value
            .target_key
            .required_of::<proto::NamespaceDelegation>("target_key")?
            .try_into()
            .validated_of::<proto::NamespaceDelegation>("target_key")?;
        let restriction = if let Some(restriction) = value.restriction {
            restriction
                .try_into()
                .validated_of::<proto::NamespaceDelegation>("restriction")?
        } else {
            #[allow(deprecated, reason = "support legacy restrictions")]
            let is_root_delegation = value.is_root_delegation;

            if is_root_delegation {
                Restriction::can_sign_all_mappings()
            } else {
                Restriction::can_sign_all_but_namespace_delegations()
            }
        };
        Self::new(namespace, target_key, restriction)
            .map_err(|error| {
                let field_name = match error {
                    NsdError::EmptyMappings | NsdError::RootCantDelegate => "restriction",
                    NsdError::SigningPublicKeyError(_) => "target_key",
                };
                InvalidProtoFieldValue::new_of::<proto::NamespaceDelegation>(field_name, error)
            })
            .map_err(Into::into)
    }
}

/// Restricts a key to only sign transactions with the specified mapping types.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Restriction {
    CanSignAllMappings(CanSignAllMappings),
    CanSignAllButNamespaceDelegations(CanSignAllButNamespaceDelegations),
    CanSignSpecificMappings(CanSignSpecificMappings),
}

impl Restriction {
    /// Create a restriction of a delegate to sign all mappings
    pub const fn can_sign_all_mappings() -> Self {
        Self::CanSignAllMappings(CanSignAllMappings {})
    }

    /// Create a restriction of a delegate to sign all mappings but namespace delegations
    pub const fn can_sign_all_but_namespace_delegations() -> Self {
        Self::CanSignAllButNamespaceDelegations(CanSignAllButNamespaceDelegations {})
    }

    /// Create a restriction of a delegate to sign only specified mappings
    ///
    /// Duplicate mapping codes are silently ignored.
    pub fn can_sign_specific_mappings(
        mappings: impl IntoIterator<Item = TopologyMappingCode>,
    ) -> Result<Self, NsdError> {
        Ok(Self::CanSignSpecificMappings(CanSignSpecificMappings::new(
            BTreeSet::from_iter(mappings),
        )?))
    }

    /// Returns `true` if the delegate can sign other namespace delegations.
    pub fn can_delegate(&self) -> bool {
        match self {
            Restriction::CanSignAllMappings(_) => true,
            Restriction::CanSignAllButNamespaceDelegations(_) => false,
            Restriction::CanSignSpecificMappings(inner) => inner.can_delegate(),
        }
    }
}

impl From<Restriction> for proto::namespace_delegation::Restriction {
    fn from(value: Restriction) -> Self {
        match value {
            Restriction::CanSignAllMappings(inner) => Self::CanSignAllMappings(inner.into()),
            Restriction::CanSignAllButNamespaceDelegations(inner) => {
                Self::CanSignAllButNamespaceDelegations(inner.into())
            }
            Restriction::CanSignSpecificMappings(inner) => {
                Self::CanSignSpecificMapings(inner.into())
            }
        }
    }
}

impl TryFrom<proto::namespace_delegation::Restriction> for Restriction {
    type Error = ConvertError;

    fn try_from(value: proto::namespace_delegation::Restriction) -> Result<Self, Self::Error> {
        match value {
            proto::namespace_delegation::Restriction::CanSignAllMappings(inner) => {
                inner.try_into().map(Self::CanSignAllMappings)
            }
            proto::namespace_delegation::Restriction::CanSignAllButNamespaceDelegations(inner) => {
                inner
                    .try_into()
                    .map(Self::CanSignAllButNamespaceDelegations)
            }
            proto::namespace_delegation::Restriction::CanSignSpecificMapings(inner) => {
                inner.try_into().map(Self::CanSignSpecificMappings)
            }
        }
    }
}

/// The key can sign all currently known mappings and all mappings that will be added in future
/// releases
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CanSignAllMappings {}

impl From<CanSignAllMappings> for proto::namespace_delegation::CanSignAllMappings {
    fn from(_: CanSignAllMappings) -> Self {
        Self {}
    }
}

impl TryFrom<proto::namespace_delegation::CanSignAllMappings> for CanSignAllMappings {
    type Error = ConvertError;

    fn try_from(_: proto::namespace_delegation::CanSignAllMappings) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

/// The key can sign all currently known mappings and all mappings that will be added in future
/// releases, except for namespace delegations
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CanSignAllButNamespaceDelegations {}

impl From<CanSignAllButNamespaceDelegations>
    for proto::namespace_delegation::CanSignAllButNamespaceDelegations
{
    fn from(_: CanSignAllButNamespaceDelegations) -> Self {
        Self {}
    }
}

impl TryFrom<proto::namespace_delegation::CanSignAllButNamespaceDelegations>
    for CanSignAllButNamespaceDelegations
{
    type Error = ConvertError;

    fn try_from(
        _: proto::namespace_delegation::CanSignAllButNamespaceDelegations,
    ) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

/// The key can only sign the explicitly specified mappings
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CanSignSpecificMappings {
    mappings: BTreeSet<TopologyMappingCode>,
}

impl CanSignSpecificMappings {
    pub fn new(mappings: BTreeSet<TopologyMappingCode>) -> Result<Self, NsdError> {
        if mappings.is_empty() {
            return Err(NsdError::EmptyMappings);
        }

        Ok(Self { mappings })
    }

    /// Mappings which this key is allowed to sign
    pub fn mappings(&self) -> &BTreeSet<TopologyMappingCode> {
        &self.mappings
    }

    /// Returns `true` if the delegate can sign other namespace delegations.
    pub fn can_delegate(&self) -> bool {
        self.mappings
            .contains(&TopologyMappingCode::NamespaceDelegation)
    }
}

impl From<CanSignSpecificMappings> for proto::namespace_delegation::CanSignSpecificMappings {
    fn from(value: CanSignSpecificMappings) -> Self {
        Self {
            mappings: value.mappings.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<proto::namespace_delegation::CanSignSpecificMappings> for CanSignSpecificMappings {
    type Error = ConvertError;

    /// This silently drops duplicate mappings
    fn try_from(
        value: proto::namespace_delegation::CanSignSpecificMappings,
    ) -> Result<Self, Self::Error> {
        let mappings = value
            .mappings
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<BTreeSet<_>, _>>()?;
        Self::new(mappings)
            .map_err(|error| {
                let field_name = match error {
                    NsdError::EmptyMappings => "mappings",
                    // Only the variants above are returned from Self::new
                    _ => unreachable!()
                };
                InvalidProtoFieldValue::new_of::<proto::namespace_delegation::CanSignSpecificMappings>(field_name, error)
            })
            .map_err(Into::into)
    }
}

/// Namespace delegation error
#[derive(Debug, thiserror::Error)]
pub enum NsdError {
    #[error("bad restrictions: no mappings specified")]
    EmptyMappings,

    #[error("bad restrictions: root certificate must be able to sign namespace delegations")]
    RootCantDelegate,

    #[error(transparent)]
    SigningPublicKeyError(#[from] SigningPublicKeyError),
}
