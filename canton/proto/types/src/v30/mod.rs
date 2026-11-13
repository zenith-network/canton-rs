use canton_crypto::v30::Signature;
use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::NonEmpty;

use crate::error::ConvertError;

pub mod mappings;

/// Topology change operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TopologyChangeOp {
    /// Adds a new or replaces an existing mapping
    AddReplace,
    /// Remove an existing mapping
    Remove,
}

impl From<TopologyChangeOp> for proto::enums::TopologyChangeOp {
    fn from(value: TopologyChangeOp) -> Self {
        match value {
            TopologyChangeOp::AddReplace => Self::AddReplace,
            TopologyChangeOp::Remove => Self::Remove,
        }
    }
}

impl From<TopologyChangeOp> for i32 {
    fn from(value: TopologyChangeOp) -> Self {
        proto::enums::TopologyChangeOp::from(value).into()
    }
}

/// Signed topology transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedTopologyTransaction {
    transaction: Vec<u8>,
    signatures: Vec<Signature>,
    proposal: bool,
    multi_transaction_signatures: Vec<MultiTransactionSignatures>,
}

impl SignedTopologyTransaction {
    pub fn new(
        transaction: Vec<u8>,
        signatures: Vec<Signature>,
        proposal: bool,
        multi_transaction_signatures: Vec<MultiTransactionSignatures>,
    ) -> Result<Self, TopologyTransactionError> {
        // TODO: check at least one signature is present
        Ok(Self {
            transaction,
            signatures,
            proposal,
            multi_transaction_signatures,
        })
    }

    /// Serialized topology transaction (protobuf bytestring)
    pub fn transaction(&self) -> &[u8] {
        &self.transaction
    }

    /// Multiple signatures
    ///
    /// Either this OR the
    /// [`multi_transaction_signatures`][Self::multi_transaction_signatures()] field always contains
    /// at least one signature
    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    /// If true, this transaction is just a proposal. this means that every signature is valid,
    /// but the signatures are insufficient to properly authorize the transaction.
    /// proposals are distributed via the topology channel too. proposals will be pruned automatically
    /// when the nodes are pruned
    pub fn proposal(&self) -> bool {
        self.proposal
    }

    /// If set, the transaction may be authorized by signing a hash computed from multiple transaction hashes
    /// This allows to effectively authorize multiple transactions with a single signature.
    /// Each item MUST contain the hash of this transaction
    pub fn multi_transaction_signatures(&self) -> &[MultiTransactionSignatures] {
        &self.multi_transaction_signatures
    }
}

impl TryFrom<proto::SignedTopologyTransaction> for SignedTopologyTransaction {
    type Error = ConvertError;

    fn try_from(value: proto::SignedTopologyTransaction) -> Result<Self, Self::Error> {
        todo!()
    }
}

/// Used in SignedTopologyTransaction to bundle together multiple transaction hashes
/// Allows submitters to only sign a single combined hash to authorize multiple transactions at once
/// The combined hash is computed from the transaction hashes
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiTransactionSignatures {
    transaction_hashes: Vec<Vec<u8>>,
    signatures: NonEmpty<Signature>,
}

impl MultiTransactionSignatures {
    pub fn new(transaction_hashes: Vec<Vec<u8>>, signatures: NonEmpty<Signature>) -> Self {
        Self {
            transaction_hashes,
            signatures,
        }
    }

    /// List of topology transaction hashes.
    pub fn transaction_hashes(&self) -> &[Vec<u8>] {
        &self.transaction_hashes
    }

    /// Signatures on the combined hash computed over the transaction_hashes
    /// MUST contain at least one
    pub fn signatures(&self) -> &NonEmpty<Signature> {
        &self.signatures
    }
}

impl TryFrom<proto::MultiTransactionSignatures> for MultiTransactionSignatures {
    type Error = ConvertError;

    fn try_from(value: proto::MultiTransactionSignatures) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TopologyTransactionError {
    #[error("topology transaction has no signatures")]
    NoSignatures,
}
