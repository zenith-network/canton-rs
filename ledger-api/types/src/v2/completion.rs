use canton_types::{LedgerString, NonEmpty, PartyId, UserId};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::InvalidProtoField as _;

/// A completion represents the status of a submitted command on the ledger: it can be successful or
/// failed.
#[derive(Clone, Debug)]
pub struct Completion {
    /// The ID of the succeeded or failed command.
    pub command_id: LedgerString,

    /// Identifies the exact type of the error.
    /// It uses the same format of conveying error details as it is used for the RPC responses of the APIs.
    pub status: Option<()>,

    /// The update_id of the transaction or reassignment that resulted from the command with command_id.
    ///
    /// Only set for successfully executed commands.
    pub update_id: Option<LedgerString>,

    /// The user-id that was used for the submission.
    pub user_id: UserId,

    /// The set of parties on whose behalf the commands were executed.
    /// Contains the ``act_as`` parties from ``commands.proto``
    /// filtered to the requesting parties in CompletionStreamRequest.
    /// The order of the parties need not be the same as in the submission.
    pub act_as: NonEmpty<PartyId>,

    /// The submission ID this completion refers to.
    pub submission_id: Option<LedgerString>,

    /// May be used in a subsequent CompletionStreamRequest to resume the consumption of this stream
    /// at a later time.
    ///
    /// Must be a valid absolute offset (positive integer).
    pub offset: i64,

    /// The traffic cost paid by this participant node for the confirmation request
    /// for the submitted command.
    ///
    /// Commands whose execution is rejected before their corresponding
    /// confirmation request is ordered by the synchronizer will report a paid
    /// traffic cost of zero.
    /// If a confirmation request is ordered for a command, but the request fails
    /// (e.g., due to contention with a concurrent contract archival), the traffic
    /// cost is paid and reported on the failed completion for the request.
    ///
    /// If you want to correlate the traffic cost of a successful completion
    /// with the transaction that resulted from the command, you can use the
    /// ``offset`` field to retrieve the transaction using
    /// ``UpdateService.GetUpdateByOffset`` on the same participant node; or alternatively use the ``update_id``
    /// field to retrieve the transaction using ``UpdateService.GetUpdateById`` on any participant node
    /// that sees the transaction.
    ///
    /// Note: for completions processed before the participant started serving
    /// traffic cost on the Ledger API, this field will be set to zero.
    /// Additionally, the total cost incurred by the submitting node for the submission of the transaction may be greater
    /// than the reported cost, for example if retries were issued due to failed submissions to the synchronizer.
    /// The cost reported here is the one paid for ordering the confirmation request.
    pub paid_traffic_cost: i64,
    // TODO: implement missing fields
}

impl TryFrom<proto::Completion> for Completion {
    type Error = ValueError;

    fn try_from(value: proto::Completion) -> Result<Self, Self::Error> {
        let mut act_as = value
            .act_as
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                PartyId::new(p)
                    .validated_of::<proto::Completion>("act_as")
                    .with_msg_owned(format!("failed to convert act_as[{idx}]"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let tail = act_as
            .pop()
            .ok_or_else(|| ValueError::raw_message("expected non-empty list"))
            .validated_of::<proto::Completion>("act_as")
            .no_msg()?;
        let act_as = NonEmpty { base: act_as, tail };

        Ok(Self {
            command_id: LedgerString::new(value.command_id)
                .validated_of::<proto::Completion>("command_id")
                .no_msg()?,
            status: value.status.map(|_| ()),
            update_id: (!value.update_id.is_empty())
                .then(|| LedgerString::new(value.update_id))
                .transpose()
                .validated_of::<proto::Completion>("update_id")
                .no_msg()?,
            user_id: UserId::new(value.user_id)
                .validated_of::<proto::Completion>("user_id")
                .no_msg()?,
            act_as,
            submission_id: (!value.submission_id.is_empty())
                .then(|| LedgerString::new(value.submission_id))
                .transpose()
                .validated_of::<proto::Completion>("submission_id")
                .no_msg()?,
            offset: value.offset,
            paid_traffic_cost: value.paid_traffic_cost,
        })
    }
}
