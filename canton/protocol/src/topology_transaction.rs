use canton_crypto::v30::{HashPurpose as _, Signature, hash_purpose::TopologyTransactionSignature};
use canton_proto::com::digitalasset::canton::protocol::v30 as proto_v30;
use canton_types::NonEmpty;
use multihash::Multihash;
use multihash_codetable::{Code, MultihashDigest as _};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue, UnspecifiedEnumValue};

use crate::error::ProtoDecodeError;

/// Topology change operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TopologyChangeOp {
    /// Adds a new or replaces an existing topology mapping
    Replace,

    /// Remove an existing topology mapping
    Remove,
}

impl TopologyChangeOp {
    pub fn into_proto_v30(self) -> proto_v30::enums::TopologyChangeOp {
        match self {
            TopologyChangeOp::Replace => proto_v30::enums::TopologyChangeOp::AddReplace,
            TopologyChangeOp::Remove => proto_v30::enums::TopologyChangeOp::Remove,
        }
    }

    pub fn from_proto_v30(
        proto: proto_v30::enums::TopologyChangeOp,
    ) -> Result<Self, UnspecifiedEnumValue> {
        match proto {
            proto_v30::enums::TopologyChangeOp::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto_v30::enums::TopologyChangeOp::AddReplace => Ok(Self::Replace),
            proto_v30::enums::TopologyChangeOp::Remove => Ok(Self::Remove),
        }
    }

    pub fn into_proto_v30_primitive(self) -> i32 {
        self.into_proto_v30().into()
    }

    pub fn from_proto_v30_primitive(value: i32) -> Result<Self, ProtoDecodeError> {
        let proto = proto_v30::enums::TopologyChangeOp::try_from(value)?;
        Self::from_proto_v30(proto).map_err(Into::into)
    }
}

/// Signed topology transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedTopologyTransaction {
    transaction: Vec<u8>,
    // Hash doesn't get serialized, but since we calculate it in the constructor anyway,
    // we can cache it here, just in case if user needs it
    hash: Multihash<64>,
    signatures: Vec<Signature>,
    proposal: bool,
    multi_transaction_signatures: Vec<MultiTransactionSignatures>,
}

impl SignedTopologyTransaction {
    /// Create new signed topology transaction
    pub fn new(
        transaction: Vec<u8>,
        signatures: Vec<Signature>,
        proposal: bool,
        multi_transaction_signatures: Vec<MultiTransactionSignatures>,
    ) -> Result<Self, TopologyTransactionError> {
        if transaction.is_empty() {
            return Err(TopologyTransactionError {
                inner: ErrorInner::EmptyTransaction,
            });
        }

        if signatures.is_empty() && multi_transaction_signatures.is_empty() {
            return Err(TopologyTransactionError {
                inner: ErrorInner::NoSignatures,
            });
        }

        let mut input = TopologyTransactionSignature::ID.to_be_bytes().to_vec();
        input.extend_from_slice(&transaction);
        let hash = Code::Sha2_256.digest(&input);

        let all_contains = multi_transaction_signatures.iter().all(|mts| {
            mts.transaction_hashes()
                .iter()
                .find(|mts_hash| *mts_hash == &hash)
                .is_some()
        });

        if !all_contains {
            return Err(TopologyTransactionError {
                inner: ErrorInner::BadMultiTxSignatures,
            });
        }

        Ok(Self {
            transaction,
            signatures,
            proposal,
            multi_transaction_signatures,
            hash,
        })
    }

    /// Serialized topology transaction (protobuf bytestring)
    pub fn transaction(&self) -> &[u8] {
        &self.transaction
    }

    /// Hash of the transaction
    pub fn transaction_hash(&self) -> Multihash<64> {
        self.hash
    }

    /// Multiple signatures
    ///
    /// Either this OR the
    /// [`multi_transaction_signatures`][Self::multi_transaction_signatures()] field always contains
    /// at least one signature.
    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    /// If `true`, this transaction is just a proposal.
    ///
    /// This means that every signature is valid, but the signatures are insufficient to properly
    /// authorize the transaction. Proposals are distributed via the topology channel too. Proposals
    /// will be pruned automatically when the nodes are pruned.
    pub fn proposal(&self) -> bool {
        self.proposal
    }

    /// If set, the transaction may be authorized by signing a hash computed from multiple
    /// transaction hashes.
    ///
    /// This allows to effectively authorize multiple transactions with a single signature.
    /// Each item MUST contain the hash of this transaction
    pub fn multi_transaction_signatures(&self) -> &[MultiTransactionSignatures] {
        &self.multi_transaction_signatures
    }

    pub fn into_proto_v30(self) -> proto_v30::SignedTopologyTransaction {
        proto_v30::SignedTopologyTransaction {
            transaction: self.transaction,
            signatures: self.signatures.into_iter().map(Into::into).collect(),
            proposal: self.proposal,
            multi_transaction_signatures: self
                .multi_transaction_signatures
                .into_iter()
                .map(MultiTransactionSignatures::into_proto_v30)
                .collect(),
        }
    }

    pub fn from_proto_v30(
        proto: proto_v30::SignedTopologyTransaction,
    ) -> Result<Self, ProtoDecodeError> {
        type Proto = proto_v30::SignedTopologyTransaction;

        let signatures = proto
            .signatures
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
            .validated_of::<Proto>("signatures")?;
        let multi_transaction_signatures = proto
            .multi_transaction_signatures
            .into_iter()
            .map(MultiTransactionSignatures::from_proto_v30)
            .collect::<Result<_, _>>()
            .validated_of::<Proto>("multi_transaction_signatures")?;
        Self::new(
            proto.transaction,
            signatures,
            proto.proposal,
            multi_transaction_signatures,
        )
        .map_err(|error| match &error.inner {
            ErrorInner::EmptyTransaction => {
                InvalidProtoFieldValue::new_of::<Proto>("transaction", error).into()
            }
            ErrorInner::NoSignatures => ProtoDecodeError::other(error),
            ErrorInner::BadMultiTxSignatures => {
                InvalidProtoFieldValue::new_of::<Proto>("multi_transaction_signatures", error)
                    .into()
            }
        })
    }
}

/// Used in [`SignedTopologyTransaction`] to bundle together multiple transaction hashes
///
/// Allows submitters to only sign a single combined hash to authorize multiple transactions
/// at once. The combined hash is computed from the transaction hashes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiTransactionSignatures {
    transaction_hashes: NonEmpty<Multihash<64>>,
    signatures: NonEmpty<Signature>,
}

impl MultiTransactionSignatures {
    pub fn new(
        transaction_hashes: NonEmpty<Multihash<64>>,
        signatures: NonEmpty<Signature>,
    ) -> Self {
        Self {
            transaction_hashes,
            signatures,
        }
    }

    /// List of topology transaction hashes.
    pub fn transaction_hashes(&self) -> &NonEmpty<Multihash<64>> {
        &self.transaction_hashes
    }

    /// Signatures on the combined hash computed over the `transaction_hashes`.
    pub fn signatures(&self) -> &NonEmpty<Signature> {
        &self.signatures
    }

    pub fn into_proto_v30(self) -> proto_v30::MultiTransactionSignatures {
        proto_v30::MultiTransactionSignatures {
            transaction_hashes: self
                .transaction_hashes
                .into_iter()
                .map(Into::into)
                .collect(),
            signatures: self.signatures.into_iter().map(Into::into).collect(),
        }
    }

    pub fn from_proto_v30(
        proto: proto_v30::MultiTransactionSignatures,
    ) -> Result<Self, ProtoDecodeError> {
        type Proto = proto_v30::MultiTransactionSignatures;

        let transaction_hashes_vec = proto
            .transaction_hashes
            .iter()
            .map(AsRef::as_ref)
            .map(Multihash::from_bytes)
            .collect::<Result<Vec<_>, _>>()
            .validated_of::<Proto>("transaction_hashes")?;
        let transaction_hashes = NonEmpty::try_from(transaction_hashes_vec)
            .validated_of::<Proto>("transaction_hashes")?;

        let signatures_vec = proto
            .signatures
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
            .validated_of::<Proto>("signatures")?;
        let signatures = NonEmpty::try_from(signatures_vec).validated_of::<Proto>("signatures")?;

        Ok(Self::new(transaction_hashes, signatures))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TopologyTransactionError {
    inner: ErrorInner,
}

#[derive(Debug, thiserror::Error)]
enum ErrorInner {
    #[error("topology transaction is empty")]
    EmptyTransaction,
    #[error("signed topology transaction has no signatures")]
    NoSignatures,
    #[error("multi transaction signatures does not contain the hash of the transaction")]
    BadMultiTxSignatures,
}
