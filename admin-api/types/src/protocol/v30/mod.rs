use canton_proto::com::digitalasset::canton::protocol::v30 as proto;

use crate::crypto::v30::Signature;

pub mod mappings;

/// Topology change operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// Serialized topology transaction (protobuf bytestring)
    pub transaction: Vec<u8>,

    /// Multiple signatures
    ///
    /// Either this field OR the multi_transaction_signatures field MUST contain at least one signature
    pub signatures: Vec<Signature>,

    /// if true, this transaction is just a proposal. this means that every signature is valid,
    /// but the signatures are insufficient to properly authorize the transaction.
    /// proposals are distributed via the topology channel too. proposals will be pruned automatically
    /// when the nodes are pruned
    pub proposal: bool,

    /// If set, the transaction may be authorized by signing a hash computed from multiple transaction hashes
    /// This allows to effectively authorize multiple transactions with a single signature.
    /// Each item MUST contain the hash of this transaction
    /// Optional
    pub multi_transaction_signatures: Vec<MultiTransactionSignatures>,
}

impl TryFrom<proto::SignedTopologyTransaction> for SignedTopologyTransaction {
    type Error = ();

    fn try_from(value: proto::SignedTopologyTransaction) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiTransactionSignatures {}
