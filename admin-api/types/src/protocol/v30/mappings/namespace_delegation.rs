use std::collections::BTreeSet;

use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::topology::Namespace;

use crate::{crypto::v30::SigningPublicKey, protocol::v30::mappings::TopologyMappingCode};

/// Namespace delegation (NSD)
///
/// Equivalent to X509v3 CA root or intermediate CAs.
///
/// If `is_root_delegation == false`, the target key may sign all mappings requiring a signature
/// for the namespace except other namespace delegation mappings.
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
    /// Fingerprint of the root key defining the namespace
    pub namespace: Namespace,

    /// Target key of getting full rights on the namespace
    ///
    /// If `target == namespace`, it's a root certificate
    pub target_key: Option<SigningPublicKey>,

    /// Restricts `target_key` to only sign transactions with the specified mapping types.
    ///
    /// For backwards compatibility, only the following combinations are valid:
    ///
    /// - `is_root_delegation = true`, `restriction = empty`: the key can sign all mappings
    /// - `is_root_delegation = false`, `restriction = empty`: the key can sign all mappings but
    ///   namespace delegations
    /// - `is_root_delegation = false`, `restriction = non-empty`: the key can only sign the mappings
    ///   according the restriction that is set
    pub restriction: Restriction,
}

impl NamespaceDelegation {
    pub fn is_root_delegation(&self) -> bool {
        todo!()
    }
}

impl From<NamespaceDelegation> for proto::NamespaceDelegation {
    fn from(value: NamespaceDelegation) -> Self {
        todo!()
    }
}

impl TryFrom<proto::NamespaceDelegation> for NamespaceDelegation {
    type Error = ();

    fn try_from(value: proto::NamespaceDelegation) -> Result<Self, Self::Error> {
        todo!()
    }
}

/// Restricts a key to only sign transactions with the specified mapping types.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Restriction {
    /// The key can sign all currently known mappings and all mappings that will be added in future
    /// releases
    CanSignAllMappings,

    /// The key can sign all currently known mappings and all mappings that will be added in future
    /// releases, except for namespace delegations
    CanSignAllButNamespaceDelegations,

    /// The key can only sign the explicitly specified mappings
    CanSignSpecificMapings {
        /// Mappings which this key is allowed to sign
        mappings: BTreeSet<TopologyMappingCode>,
    },
}

impl From<Restriction> for proto::namespace_delegation::Restriction {
    fn from(value: Restriction) -> Self {
        todo!()
    }
}

impl TryFrom<proto::namespace_delegation::Restriction> for Restriction {
    type Error = ();

    fn try_from(value: proto::namespace_delegation::Restriction) -> Result<Self, Self::Error> {
        todo!()
    }
}
