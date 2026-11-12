use std::collections::BTreeSet;

use canton_crypto::v30::{HashPurpose as _, hash_purpose::DecentralizedNamespace};
use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::{NonZeroU31, topology::Namespace};
use multihash_codetable::{Code, MultihashDigest as _};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue};

use crate::error::ValueError;

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
    decentralized_namespace: Namespace,
    threshold: NonZeroU31,
    owners: BTreeSet<Namespace>,
}

impl DecentralizedNamespaceDefinition {
    /// Create new decentralized namespace definition
    ///
    /// Duplicate owners are silently ignored.
    ///
    /// Error is returned if `threshold` is greater than the number of distinct owners or is a
    /// total number of owners cannot be represented as [`NonZeroU31`].
    ///
    /// Note that this function doesn't check that DND fingerprint matches the owners in any way.
    /// The caller is responsible for providing a good `decentralized_namespace`. It could be an
    /// already existing DND namespace with modified owners or a new one, calculated from owners.
    ///
    /// If you are creating a new DND, prefer using [`Self::initial`] or
    /// [`Self::initial_single_owner`].
    pub fn new(
        decentralized_namespace: Namespace,
        threshold: NonZeroU31,
        owners: impl IntoIterator<Item = Namespace>,
    ) -> Result<Self, DndError> {
        let owners = BTreeSet::from_iter(owners);
        Self::validate(threshold, &owners)?;
        Ok(Self {
            decentralized_namespace,
            threshold,
            owners,
        })
    }

    fn validate(threshold: NonZeroU31, owners: &BTreeSet<Namespace>) -> Result<(), DndError> {
        let owners_len = match NonZeroU31::try_from(owners.len()) {
            Ok(len) => len,
            Err(_) => return Err(DndError::TooManyOwners),
        };
        if threshold > owners_len {
            return Err(DndError::ThresholdTooBig);
        }
        Ok(())
    }

    /// Create a new initial DND, computing the namespace from owners (see
    /// [`Self::compute_namespace`]).
    ///
    /// Duplicate owners are silently ignored.
    ///
    /// Error is returned, if threshold is greater than the number of distinct owners.
    pub fn initial(
        threshold: NonZeroU31,
        owners: impl IntoIterator<Item = Namespace>,
    ) -> Result<Self, DndError> {
        let owners = BTreeSet::from_iter(owners);
        Self::validate(threshold, &owners)?;
        let decentralized_namespace = Self::compute_namespace_inner(&owners);
        Ok(Self {
            decentralized_namespace,
            threshold,
            owners,
        })
    }

    /// Same as [`Self::initial`], but use a single owner. Threshold is set to `1`.
    pub fn initial_single_owner(owner: Namespace) -> Self {
        let owners = BTreeSet::from_iter([owner]);
        let decentralized_namespace = Self::compute_namespace_inner(&owners);
        Self {
            decentralized_namespace,
            threshold: NonZeroU31::MIN,
            owners,
        }
    }

    /// Compute the namespace of the DND based on provided owners.
    ///
    /// Duplicate owners are silently ignored. Input order doesn't affect the resulting namespace.
    ///
    /// This implementation matches Canton computation of a DND namespace. The caller is responsible
    /// for providing valid namespaces.
    pub fn compute_namespace(owners: impl IntoIterator<Item = Namespace>) -> Namespace {
        // Ordering of Namespace-s is aligned with Canton
        let owners = BTreeSet::from_iter(owners);
        Self::compute_namespace_inner(&owners)
    }

    fn compute_namespace_inner(owners: &BTreeSet<Namespace>) -> Namespace {
        let mut input = DecentralizedNamespace::ID.to_be_bytes().to_vec();
        for owner in owners {
            // For each owner we write 'owner_bytes.length || owner_bytes'
            // Valid namespace strings contain at most 68 bytes, so it's safe to cast here
            input.extend_from_slice(&(owner.len() as i32).to_be_bytes());
            input.extend_from_slice(owner.as_bytes());
        }

        let hash = Code::Sha2_256.digest(&input);
        let hex_str = hex::encode(hash.to_bytes());

        debug_assert_eq!(hex_str.len(), 68);
        // 68-digit hex string is always a valid namespace, so it's safe to use new_unchecked here
        Namespace::new_unchecked(hex_str)
    }

    /// Name of the decentralized namespace, computed from the hash of its initial owners
    pub fn decentralized_namespace(&self) -> &Namespace {
        &self.decentralized_namespace
    }

    /// The threshold required for any subsequent update signing
    pub fn threshold(&self) -> NonZeroU31 {
        self.threshold
    }

    /// The namespaces of the owners
    pub fn owners_iter(&self) -> impl Iterator<Item = &Namespace> {
        self.owners.iter()
    }

    /// Add owner to the DND. Returns `true` if inserted owner wasn't in the owners list before, and
    /// `false` otherwise.
    ///
    /// Note that this doesn't affect the namespace of the DND.
    ///
    /// Error is returned, if the number of owners becomes too big.
    #[must_use]
    pub fn add_owner(&mut self, owner: Namespace) -> Result<bool, DndError> {
        // Note: this could only overflow and cause invalid behavior on platforms where
        // usize::MAX is less than i32::MAX (16-bit platforms), so we don't really care about that.
        if NonZeroU31::try_from(self.owners.len() + 1).is_err() {
            return Err(DndError::TooManyOwners);
        };

        Ok(self.owners.insert(owner))
    }

    /// Set a new threshold for the DND
    ///
    /// Error is returned if new threshold is greater than the number of owners.
    pub fn set_threshold(&mut self, threshold: NonZeroU31) -> Result<(), DndError> {
        Self::validate(threshold, &self.owners)?;
        self.threshold = threshold;
        Ok(())
    }
}

impl From<DecentralizedNamespaceDefinition> for proto::DecentralizedNamespaceDefinition {
    fn from(value: DecentralizedNamespaceDefinition) -> Self {
        Self {
            decentralized_namespace: value.decentralized_namespace.into(),
            threshold: value.threshold.into(),
            owners: value.owners.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<proto::DecentralizedNamespaceDefinition> for DecentralizedNamespaceDefinition {
    type Error = ValueError;

    fn try_from(value: proto::DecentralizedNamespaceDefinition) -> Result<Self, Self::Error> {
        let decentralized_namespace = Namespace::new(value.decentralized_namespace)
            .validated_of::<proto::DecentralizedNamespaceDefinition>(
            "decentralized_namespace",
        )?;
        let threshold = value
            .threshold
            .try_into()
            .validated_of::<proto::DecentralizedNamespaceDefinition>("threshold")?;
        let owners = value
            .owners
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
            .validated_of::<proto::DecentralizedNamespaceDefinition>("owners")?
            .into_iter();

        Self::new(decentralized_namespace, threshold, owners)
            .map_err(|error| {
                let field_name = match error {
                    DndError::TooManyOwners => "owners",
                    DndError::ThresholdTooBig => "threshold",
                };
                InvalidProtoFieldValue::new_of::<proto::DecentralizedNamespaceDefinition>(
                    field_name, error,
                )
            })
            .map_err(Into::into)
    }
}

/// Decentralized namespace definition error
#[derive(Debug, thiserror::Error)]
pub enum DndError {
    #[error(
        "number of owners is too big (must fit into {}..={})",
        NonZeroU31::MIN,
        NonZeroU31::MAX
    )]
    TooManyOwners,

    #[error("threshold must not be greater than number of owners")]
    ThresholdTooBig,
}
