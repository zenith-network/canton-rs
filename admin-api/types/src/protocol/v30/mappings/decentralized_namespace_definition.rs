use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::topology::Namespace;

/// A decentralized namespace definition (DND) that creates a new namespace supported by the
/// the original owners
///
/// ## Authorization
///
/// The decentralized namespace definition with `serial = 1` must be authorized by all the owners of
/// the namespace that form the decentralized namespace. For definitions with `serial > 1`, we need
/// the authorization of `threshold` owners plus all new owners
///
/// Any further transaction within the decentralized namespace other than decentralized namespace
/// definitions needs `threshold` signatures of the owners
///
/// ## Uniqueness key
///
/// ```plaintext
/// decentralized_namespace
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DecentralizedNamespaceDefinition {
    /// Name of the decentralized namespace, computed from the hash of its initial owners
    pub decentralized_namespace: Namespace,

    /// The threshold required for any subsequent update signing
    pub threshold: i32,

    /// The namespaces of the owners
    pub owners: Vec<Namespace>,
}

impl From<DecentralizedNamespaceDefinition> for proto::DecentralizedNamespaceDefinition {
    fn from(value: DecentralizedNamespaceDefinition) -> Self {
        Self {
            decentralized_namespace: value.decentralized_namespace.into(),
            threshold: value.threshold,
            owners: value.owners.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<proto::DecentralizedNamespaceDefinition> for DecentralizedNamespaceDefinition {
    type Error = ();

    fn try_from(value: proto::DecentralizedNamespaceDefinition) -> Result<Self, Self::Error> {
        todo!()
    }
}
