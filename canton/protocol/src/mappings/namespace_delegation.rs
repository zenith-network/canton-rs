use std::collections::BTreeSet;

use canton_crypto::v30::{EncodedSigningPublicKey, SigningPublicKeyError};
use canton_proto::com::digitalasset::canton::protocol::v30;
use canton_types::topology::Namespace;
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue, RequiredProtoField as _};

use crate::{
    TopologyMappingCode, TopologyMappingVariant,
    error::{ProtoDecodeError, ProtoEncodeError, UnexpectedTopologyMapping},
    mappings::TopologyMapping,
};

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
    /// Code of the topology mapping
    pub const CODE: TopologyMappingCode = TopologyMappingCode::NamespaceDelegation;

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
        // TODO: maybe we should require Namespace key usage as well here
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

impl TopologyMapping for NamespaceDelegation {
    type ProtoV30 = v30::NamespaceDelegation;

    /// In v31 NSD represented with the same message as in v30
    type ProtoV31 = v30::NamespaceDelegation;

    fn code(&self) -> TopologyMappingCode {
        Self::CODE
    }

    fn into_variant(self) -> TopologyMappingVariant {
        TopologyMappingVariant::NamespaceDelegation(self)
    }

    fn from_variant(variant: TopologyMappingVariant) -> Result<Self, UnexpectedTopologyMapping> {
        match variant {
            TopologyMappingVariant::NamespaceDelegation(mapping) => Ok(mapping),
            other => Err(UnexpectedTopologyMapping::new(Self::CODE, other.code())),
        }
    }

    fn into_proto_v30(self) -> Result<Self::ProtoV30, ProtoEncodeError> {
        Ok(v30::NamespaceDelegation {
            namespace: self.namespace.into(),
            target_key: Some(self.target_key.into()),
            #[allow(deprecated)]
            is_root_delegation: false,
            restriction: Some(self.restriction.into_proto_v30()),
        })
    }

    fn from_proto_v30(proto: Self::ProtoV30) -> Result<Self, ProtoDecodeError> {
        type Proto = v30::NamespaceDelegation;

        let namespace = Namespace::new(proto.namespace).validated_of::<Proto>("namespace")?;
        let target_key = proto
            .target_key
            .required_of::<Proto>("target_key")?
            .try_into()
            .validated_of::<Proto>("target_key")?;
        let restriction = if let Some(restriction) = proto.restriction {
            Restriction::from_proto_v30(restriction).validated_of::<Proto>("restriction")?
        } else {
            #[allow(deprecated, reason = "support legacy restrictions")]
            let is_root_delegation = proto.is_root_delegation;

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
                InvalidProtoFieldValue::new_of::<Proto>(field_name, error)
            })
            .map_err(Into::into)
    }

    fn into_proto_v31(self) -> Result<Self::ProtoV31, ProtoEncodeError> {
        self.into_proto_v30()
    }

    fn from_proto_v31(proto: Self::ProtoV31) -> Result<Self, ProtoDecodeError> {
        Self::from_proto_v30(proto)
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

    pub fn into_proto_v30(self) -> v30::namespace_delegation::Restriction {
        type Proto = v30::namespace_delegation::Restriction;

        match self {
            Self::CanSignAllMappings(inner) => Proto::CanSignAllMappings(inner.into_proto_v30()),
            Self::CanSignAllButNamespaceDelegations(inner) => {
                Proto::CanSignAllButNamespaceDelegations(inner.into_proto_v30())
            }
            Self::CanSignSpecificMappings(inner) => {
                Proto::CanSignSpecificMapings(inner.into_proto_v30())
            }
        }
    }

    pub fn from_proto_v30(
        proto: v30::namespace_delegation::Restriction,
    ) -> Result<Self, ProtoDecodeError> {
        type Proto = v30::namespace_delegation::Restriction;

        match proto {
            Proto::CanSignAllMappings(inner) => {
                CanSignAllMappings::from_proto_v30(inner).map(Self::CanSignAllMappings)
            }
            Proto::CanSignAllButNamespaceDelegations(inner) => {
                CanSignAllButNamespaceDelegations::from_proto_v30(inner)
                    .map(Self::CanSignAllButNamespaceDelegations)
            }
            Proto::CanSignSpecificMapings(inner) => {
                CanSignSpecificMappings::from_proto_v30(inner).map(Self::CanSignSpecificMappings)
            }
        }
    }
}

/// The key can sign all currently known mappings and all mappings that will be added in future
/// releases
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CanSignAllMappings {}

impl CanSignAllMappings {
    pub fn into_proto_v30(self) -> v30::namespace_delegation::CanSignAllMappings {
        v30::namespace_delegation::CanSignAllMappings {}
    }

    pub fn from_proto_v30(
        _: v30::namespace_delegation::CanSignAllMappings,
    ) -> Result<Self, ProtoDecodeError> {
        Ok(Self {})
    }
}

/// The key can sign all currently known mappings and all mappings that will be added in future
/// releases, except for namespace delegations
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CanSignAllButNamespaceDelegations {}

impl CanSignAllButNamespaceDelegations {
    pub fn into_proto_v30(self) -> v30::namespace_delegation::CanSignAllButNamespaceDelegations {
        v30::namespace_delegation::CanSignAllButNamespaceDelegations {}
    }

    pub fn from_proto_v30(
        _: v30::namespace_delegation::CanSignAllButNamespaceDelegations,
    ) -> Result<Self, ProtoDecodeError> {
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

    pub fn into_proto_v30(self) -> v30::namespace_delegation::CanSignSpecificMappings {
        v30::namespace_delegation::CanSignSpecificMappings {
            mappings: self
                .mappings
                .into_iter()
                .map(TopologyMappingCode::into_proto_v30_primitive)
                .collect(),
        }
    }

    /// This silently drops duplicate mappings
    pub fn from_proto_v30(
        proto: v30::namespace_delegation::CanSignSpecificMappings,
    ) -> Result<Self, ProtoDecodeError> {
        type Proto = v30::namespace_delegation::CanSignSpecificMappings;

        let mappings = proto
            .mappings
            .into_iter()
            .map(TopologyMappingCode::from_proto_v30_primitive)
            .collect::<Result<BTreeSet<_>, _>>()
            .validated_of::<Proto>("mappings")?;

        Self::new(mappings)
            .map_err(|error| {
                let field_name = match error {
                    NsdError::EmptyMappings => "mappings",
                    // Only the variants above are returned from Self::new
                    _ => unreachable!(),
                };
                InvalidProtoFieldValue::new_of::<Proto>(field_name, error)
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
